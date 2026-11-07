//! Fil de travail : traite une à une, hors du fil principal, les tâches longues
//! (synchro Steam, images et infos, base LaunchBox) et signale leur progression.
//!
//! Événements : `metadata-progress` (`{ done, total, title, phase }` ; pour `launchbox-download`,
//! `done`/`total` sont en octets ; `phase: "idle"` quand plus aucune tâche n'attend) et
//! `metadata-error` (message en français) quand une tâche entière échoue.

use crate::launchbox::index;
use crate::library::Game;
use crate::metadata::{self, Source};
use crate::stores::steam::{self, StoreType};
use crate::stores::steam_web::{self, SteamItem};
use crate::{AppState, lock, media, mutate_library_if};
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Sender};
use tauri::{AppHandle, Emitter, Manager};

/// Âge maximal de la base LaunchBox avant un nouveau téléchargement automatique.
const LB_MAX_AGE_DAYS: u64 = 30;

pub enum Job {
    SyncSteam,
    RefreshMetadata { game_ids: Option<Vec<String>>, force: bool },
    SetLaunchboxMatch { game_id: String, lb_id: u64 },
    UpdateLaunchboxDb,
}

#[derive(Clone, serde::Serialize)]
struct Progress {
    done: u64,
    total: u64,
    title: Option<String>,
    phase: &'static str,
}

/// Démarre le fil de travail ; les tâches envoyées sont traitées dans l'ordre.
pub fn start(app: AppHandle) -> Sender<Job> {
    let (tx, rx) = mpsc::channel::<Job>();
    std::thread::spawn(move || {
        let mut worker =
            Worker { app, client: media::http_client(), steam_types: HashMap::new(), steam_offline: false };
        for job in rx {
            worker.steam_offline = false;
            // Une panique abandonne la tâche, pas le fil : les suivantes sont traitées.
            match panic::catch_unwind(AssertUnwindSafe(|| worker.run(job))) {
                Ok(Ok(())) => {}
                Ok(Err(e)) => worker.error(&e),
                Err(_) => worker.error("Erreur inattendue pendant une tâche de fond : elle a été abandonnée."),
            }
            // Toujours atteint : « libre » seulement quand plus aucune tâche n'attend.
            if worker.state().pending.fetch_sub(1, Ordering::SeqCst) == 1 {
                worker.progress(0, 0, None, "idle");
            }
        }
    });
    tx
}

struct Worker {
    app: AppHandle,
    client: Client,
    /// Types Steam déjà connus : évite d'interroger la boutique à chaque synchro.
    steam_types: HashMap<u32, StoreType>,
    /// La boutique n'a pas répondu pendant la tâche en cours : on ne réessaie pas avant la suivante.
    steam_offline: bool,
}

impl Worker {
    fn run(&mut self, job: Job) -> Result<(), String> {
        match job {
            Job::SyncSteam => self.sync_steam(),
            Job::RefreshMetadata { game_ids, force } => {
                self.refresh(game_ids.as_deref(), force, None);
                Ok(())
            }
            Job::SetLaunchboxMatch { game_id, lb_id } => self.set_match(&game_id, lb_id),
            Job::UpdateLaunchboxDb => {
                let platforms = metadata::lb_platforms(&lock(&self.state().library));
                self.ensure_index(&platforms, 0)
            }
        }
    }

    fn state(&self) -> &AppState {
        self.app.state::<AppState>().inner()
    }

    fn progress(&self, done: u64, total: u64, title: Option<String>, phase: &'static str) {
        let _ = self.app.emit("metadata-progress", Progress { done, total, title, phase });
    }

    fn error(&self, message: &str) {
        eprintln!("{message}");
        let _ = self.app.emit("metadata-error", message);
    }

    /// Interroge la boutique (types et URL des images) ; met à jour le cache des types.
    fn steam_items(&mut self, ids: &[u32]) -> HashMap<u32, SteamItem> {
        if self.steam_offline {
            return HashMap::new();
        }
        match steam_web::get_items(&self.client, ids) {
            Ok(items) => {
                self.steam_types.extend(items.iter().map(|(id, it)| (*id, it.kind)));
                items
            }
            Err(e) => {
                eprintln!("Boutique Steam injoignable : {e}");
                self.steam_offline = true;
                HashMap::new()
            }
        }
    }

    fn sync_steam(&mut self) -> Result<(), String> {
        let Some(root) = steam::steam_root() else { return Ok(()) };
        self.progress(0, 0, None, "steam");
        let apps = steam::installed_apps(&root);
        let playtimes = steam::playtimes(&root);
        let unknown: Vec<u32> =
            apps.iter().map(|a| a.app_id).filter(|id| !self.steam_types.contains_key(id)).collect();
        // Hors ligne : types absents → exclusion par nom dans `sync_into`.
        let items = if unknown.is_empty() { HashMap::new() } else { self.steam_items(&unknown) };
        let types = &self.steam_types;
        let report = mutate_library_if(&self.app, |lib| {
            let report = steam::sync_into(lib, &apps, &playtimes, Some(types));
            let changed = !report.added.is_empty() || report.updated > 0;
            (report, changed)
        })
        .map_err(|e| format!("Synchro Steam non enregistrée : {e}"))?;
        if !report.added.is_empty() {
            self.refresh(Some(&report.added), false, Some(items));
        }
        Ok(())
    }

    /// Récupère images et infos des jeux prévus par `plan_refresh`, un jeu à la fois.
    /// Un échec sur un jeu n'arrête pas les autres ; le jeu reste sans `metadata`.
    fn refresh(&mut self, game_ids: Option<&[String]>, force: bool, steam_items: Option<HashMap<u32, SteamItem>>) {
        let (work, platforms) = {
            let lib = lock(&self.state().library);
            let work: Vec<(Game, Source)> = metadata::plan_refresh(&lib, game_ids, force)
                .iter()
                .filter_map(|id| lib.game(id))
                .filter_map(|g| metadata::source_of(&lib, g).map(|s| (g.clone(), s)))
                .collect();
            (work, metadata::lb_platforms(&lib))
        };
        if work.is_empty() {
            return;
        }

        let steam_ids: Vec<u32> = work
            .iter()
            .filter_map(|(_, s)| match s {
                Source::Steam(id) => Some(*id),
                Source::Launchbox(_) => None,
            })
            .collect();
        let mut items = steam_items.unwrap_or_default();
        let missing: Vec<u32> = steam_ids.iter().copied().filter(|id| !items.contains_key(id)).collect();
        if !missing.is_empty() {
            items.extend(self.steam_items(&missing));
        }
        let root = if steam_ids.is_empty() { None } else { steam::steam_root() };

        let needs_lb = work.iter().any(|(_, s)| matches!(s, Source::Launchbox(_)));
        let lb_ready = needs_lb
            && self
                .ensure_index(&platforms, LB_MAX_AGE_DAYS)
                .inspect_err(|e| self.error(e))
                .is_ok();

        let total = work.len() as u64;
        let mut save_errors: Vec<String> = Vec::new();
        for (done, (game, source)) in work.iter().enumerate() {
            self.progress(done as u64, total, Some(game.title.clone()), "images");
            let dir = self.state().media_dir(&game.id);
            let fields = match source {
                Source::Steam(app_id) => {
                    let app_id = *app_id;
                    let local = root.as_deref().map(|r| steam_web::local_cache(r, app_id)).unwrap_or_default();
                    metadata::fetch_steam(&self.client, app_id, items.get(&app_id), local, &dir)
                }
                Source::Launchbox(platform) if lb_ready => {
                    let found = {
                        let slot = self.state().loaded_index();
                        slot.as_ref()
                            .and_then(|idx| metadata::find_launchbox(&idx.games, game, platform))
                            .map(|(lb, locked)| (lb.clone(), locked))
                    };
                    found.and_then(|(lb, locked)| metadata::fetch_launchbox(&self.client, &lb, locked, &dir))
                }
                Source::Launchbox(_) => None,
            };
            match fields {
                Some(f) => {
                    if let Err(e) = self.apply(&game.id, f, false) {
                        eprintln!("{e}");
                        save_errors.push(e);
                    }
                }
                None => eprintln!("Pas d'images ni d'infos pour « {} » (nouvel essai plus tard)", game.title),
            }
        }
        self.progress(total, total, None, "images");
        if let Some(first) = save_errors.first() {
            self.error(&format!("Images et infos non enregistrées pour {} jeu(x). {first}", save_errors.len()));
        }
    }

    /// Applique le résultat au jeu s'il existe encore (un seul enregistrement par jeu), puis
    /// supprime de son dossier les images qu'il ne référence plus.
    fn apply(&self, game_id: &str, fields: metadata::MetaFields, replace: bool) -> Result<(), String> {
        let dir = self.state().media_dir(game_id);
        let stale = mutate_library_if(&self.app, |lib| match lib.game_mut(game_id) {
            Some(g) => {
                let before = metadata::images_of(g);
                // Rien de changé (ex. mêmes images du cache local à chaque démarrage hors
                // ligne) : ni enregistrement ni nouvelle révision.
                let changed = metadata::apply(g, fields, replace);
                (metadata::stale_images(before, g, &dir), changed)
            }
            None => (Vec::new(), false),
        })
        .map_err(|e| format!("Enregistrement de la bibliothèque impossible : {e}"))?;
        for path in stale {
            if let Err(e) = fs::remove_file(&path) {
                eprintln!("Ancienne image {} non supprimée : {e}", path.display());
            }
        }
        Ok(())
    }

    /// Choix manuel d'un jeu LaunchBox : remplace images et infos (sauf le titre) et verrouille.
    /// Tout ou rien : les images sont téléchargées dans un dossier provisoire et le jeu n'est
    /// modifié que si toutes celles prévues ont été obtenues.
    fn set_match(&mut self, game_id: &str, lb_id: u64) -> Result<(), String> {
        let title = {
            let lib = lock(&self.state().library);
            lib.game(game_id).map(|g| g.title.clone()).ok_or("Jeu inconnu")?
        };
        let lb = {
            let slot = self.state().loaded_index();
            slot.as_ref().and_then(|idx| idx.games.iter().find(|g| g.id == lb_id).cloned())
        }
        .ok_or("Ce jeu n'est pas dans la base LaunchBox : mettez la base à jour.")?;
        self.progress(0, 1, Some(title.clone()), "images");
        let dir = self.state().media_dir(game_id);
        let staging = dir.join(".en-cours");
        let _ = fs::remove_dir_all(&staging);
        let fetched = (|| -> Result<metadata::MetaFields, String> {
            let mut f = metadata::fetch_launchbox(&self.client, &lb, true, &staging)
                .filter(|f| !metadata::missing_listed_images(&lb, f))
                .ok_or_else(|| {
                    format!("Les images de « {} » n'ont pas pu être téléchargées : rien n'a été changé.", lb.name)
                })?;
            metadata::commit_images(&mut f, &dir)?;
            Ok(f)
        })();
        let _ = fs::remove_dir_all(&staging);
        self.apply(game_id, fetched?, true)?;
        self.progress(1, 1, Some(title), "images");
        Ok(())
    }

    /// Garantit une base LaunchBox de moins de `max_age_days` jours (0 = retéléchargement)
    /// et un index à jour pour `platforms` : en mémoire, sinon `index.json`, sinon reconstruit.
    fn ensure_index(&self, platforms: &[String], max_age_days: u64) -> Result<(), String> {
        let state = self.state();
        let dir = state.launchbox_dir();
        let mut last = 0u64;
        let zip = index::ensure_db(&self.client, &dir, max_age_days, |done, total| {
            if done - last >= 1 << 20 || done == total {
                last = done;
                self.progress(done, total, None, "launchbox-download");
            }
        })?;
        // La base a pu être retéléchargée : le catalogue des plateformes sera relu.
        *lock(&state.lb_platforms) = None;
        let mtime = metadata::mtime_secs(&zip).unwrap_or(0);
        if state.loaded_index().as_ref().is_some_and(|idx| metadata::index_is_current(idx, mtime, platforms)) {
            return Ok(());
        }
        self.progress(0, 0, None, "launchbox-index");
        let refs: Vec<&str> = platforms.iter().map(String::as_str).collect();
        // Long (plusieurs secondes) : construit sans tenir le verrou de l'index.
        let idx = index::build_index(&zip, &refs)?;
        if let Err(e) = idx.save(&state.index_path()) {
            eprintln!("{e}");
        }
        *lock(&state.lb_index) = Some(idx);
        Ok(())
    }
}
