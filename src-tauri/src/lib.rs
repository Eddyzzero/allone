mod launcher;
mod legacy;
mod launchbox;
mod library;
mod logos;
mod media;
mod metadata;
mod scanner;
mod stores;
mod vdf;
mod worker;

use launchbox::index::{LbGame, LbIndex, LbPlatform};
use library::{Game, LaunchKind, Library};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager, State};
use worker::Job;

struct AppState {
    library: Mutex<Library>,
    /// `library.json`, dans le dossier de données de l'application.
    path: PathBuf,
    running: Mutex<HashSet<String>>,
    /// File du fil de travail (synchro Steam, images et infos).
    jobs: Sender<Job>,
    /// Tâches confiées au fil de travail et pas encore terminées (en cours comprise).
    pending: AtomicUsize,
    /// Index LaunchBox, chargé à la première utilisation.
    lb_index: Mutex<Option<LbIndex>>,
    /// Catalogue des plateformes LaunchBox (`Platforms.xml`), vidé quand la base est mise à jour.
    lb_platforms: Mutex<Option<Vec<LbPlatform>>>,
}

impl AppState {
    fn save(&self, lib: &Library) -> Result<(), String> {
        lib.save(&self.path)
    }

    fn data_dir(&self) -> &Path {
        self.path.parent().unwrap_or(Path::new("."))
    }

    /// Dossier des images d'un jeu : `media/<gameId>/`.
    fn media_dir(&self, game_id: &str) -> PathBuf {
        self.data_dir().join("media").join(game_id)
    }

    /// Dossier de la base LaunchBox (`Metadata.zip`, `index.json`).
    fn launchbox_dir(&self) -> PathBuf {
        self.data_dir().join("launchbox")
    }

    fn index_path(&self) -> PathBuf {
        self.launchbox_dir().join("index.json")
    }

    /// Index LaunchBox en mémoire, lu depuis `index.json` s'il ne l'est pas encore
    /// (reste vide si le fichier n'existe pas : jamais de téléchargement ici).
    fn loaded_index(&self) -> MutexGuard<'_, Option<LbIndex>> {
        let mut slot = lock(&self.lb_index);
        if slot.is_none() {
            *slot = LbIndex::load(&self.index_path());
        }
        slot
    }

    /// Confie une tâche au fil de travail.
    fn push(&self, job: Job) -> Result<(), String> {
        self.pending.fetch_add(1, Ordering::SeqCst);
        self.jobs.send(job).map_err(|_| {
            self.pending.fetch_sub(1, Ordering::SeqCst);
            "Le fil de travail est arrêté.".to_string()
        })
    }
}

/// Verrouille même si un autre fil a paniqué en tenant le verrou : les données restent
/// utilisables (chaque modification est complète avant d'être enregistrée).
pub(crate) fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GameEvent {
    game_id: String,
    /// Durée de la session en secondes (seulement à la fermeture).
    session_secs: u64,
    error: Option<String>,
}

#[tauri::command]
fn get_library(state: State<AppState>) -> Library {
    lock(&state.library).clone()
}

/// Modifie la bibliothèque côté Rust (tâches de fond, temps de jeu…) :
/// verrou, modification, `revision + 1`, enregistrement, puis événement
/// `library-changed` pour que l'interface se recharge.
pub(crate) fn mutate_library<R>(app: &AppHandle, f: impl FnOnce(&mut Library) -> R) -> Result<R, String> {
    mutate_library_if(app, |lib| (f(lib), true))
}

/// Comme `mutate_library`, mais `f` indique si elle a réellement modifié quelque chose :
/// sinon ni révision, ni enregistrement, ni événement.
pub(crate) fn mutate_library_if<R>(app: &AppHandle, f: impl FnOnce(&mut Library) -> (R, bool)) -> Result<R, String> {
    let state = app.state::<AppState>();
    let (result, changed) = {
        let mut lib = lock(&state.library);
        let (result, changed) = f(&mut lib);
        if changed {
            lib.revision += 1;
            state.save(&lib)?;
        }
        (result, changed)
    };
    if changed {
        let _ = app.emit("library-changed", ());
    }
    Ok(result)
}

/// Remplace toute la bibliothèque (ajout, favori, réglages…) et l'enregistre.
/// Refusée (`STALE`) si l'interface est partie d'une version périmée ;
/// renvoie la nouvelle révision.
#[tauri::command]
fn save_library(state: State<AppState>, library: Library) -> Result<u64, String> {
    let mut current = lock(&state.library);
    let next = library::accept_save(&current, library)?;
    state.save(&next)?;
    let revision = next.revision;
    *current = next;
    Ok(revision)
}

/// Indique, pour chaque jeu, si ses fichiers sont présents sur le disque.
#[tauri::command]
fn check_files(state: State<AppState>) -> HashMap<String, bool> {
    let lib = lock(&state.library);
    lib.games
        .iter()
        .map(|g| {
            let ok = launcher::plan(&lib, g)
                .and_then(|p| launcher::check(&p, g))
                .is_ok();
            (g.id.clone(), ok)
        })
        .collect()
}

/// La commande qui serait exécutée, sans lancer le jeu.
#[tauri::command]
fn preview_launch(state: State<AppState>, game_id: String) -> Result<launcher::LaunchPlan, String> {
    let lib = lock(&state.library);
    let game = lib.game(&game_id).ok_or("Jeu inconnu")?;
    launcher::plan(&lib, game)
}

#[tauri::command]
fn launch_game(app: AppHandle, state: State<AppState>, game_id: String) -> Result<launcher::LaunchPlan, String> {
    let (plan, is_store) = {
        let lib = lock(&state.library);
        let game = lib.game(&game_id).ok_or("Jeu inconnu")?;
        let plan = launcher::plan(&lib, game)?;
        launcher::check(&plan, game)?;
        (plan, game.kind == LaunchKind::Store)
    };

    if is_store {
        // Jeu de boutique : Steam le lance, on ne le suit pas (pas de temps de jeu).
        use tauri_plugin_opener::OpenerExt;
        app.opener().open_url(&plan.program, None::<&str>).map_err(|e| format!("Impossible d'ouvrir Steam : {e}"))?;
        if let Err(e) = mutate_library(&app, |lib| {
            if let Some(g) = lib.game_mut(&game_id) {
                g.last_played = Some(library::now_unix());
            }
        }) {
            eprintln!("Enregistrement de la dernière partie impossible : {e}");
        }
        let _ = app.emit("game-started", GameEvent { game_id: game_id.clone(), session_secs: 0, error: None });
        let _ = app.emit("game-exited", GameEvent { game_id, session_secs: 0, error: None });
        return Ok(plan);
    }

    if !lock(&state.running).insert(game_id.clone()) {
        return Err("Ce jeu est déjà lancé.".into());
    }

    let mut child = match plan.to_command().spawn() {
        Ok(child) => child,
        Err(e) => {
            lock(&state.running).remove(&game_id);
            return Err(format!("Impossible de lancer {} : {e}", plan.program));
        }
    };

    if let Err(e) = mutate_library(&app, |lib| {
        if let Some(g) = lib.game_mut(&game_id) {
            g.last_played = Some(library::now_unix());
        }
    }) {
        eprintln!("Enregistrement de la dernière partie impossible : {e}");
    }
    let _ = app.emit("game-started", GameEvent { game_id: game_id.clone(), session_secs: 0, error: None });

    // On attend la fin du jeu dans un fil séparé pour compter le temps de jeu.
    let started = Instant::now();
    std::thread::spawn(move || {
        let result = child.wait();
        let secs = started.elapsed().as_secs();
        lock(&app.state::<AppState>().running).remove(&game_id);
        if let Err(e) = mutate_library(&app, |lib| {
            if let Some(g) = lib.game_mut(&game_id) {
                g.playtime_secs += secs;
            }
        }) {
            eprintln!("Enregistrement du temps de jeu impossible : {e}");
        }
        let error = result.err().map(|e| e.to_string());
        let _ = app.emit("game-exited", GameEvent { game_id, session_secs: secs, error });
    });

    Ok(plan)
}

#[tauri::command]
fn scan_system(state: State<AppState>, system_id: String) -> Result<Vec<scanner::FoundGame>, String> {
    let lib = lock(&state.library);
    let system = lib.system(&system_id).ok_or("Système inconnu")?;
    Ok(scanner::scan_system(&lib, system))
}

/// Ajoute un jeu à partir d'un fichier choisi par l'utilisateur.
/// Un `.exe` (ou un exécutable sous Linux) devient un jeu natif.
#[tauri::command]
fn add_game(
    app: AppHandle,
    path: String,
    system_id: String,
    title: Option<String>,
    native: Option<bool>,
) -> Result<Game, String> {
    let file = Path::new(&path);
    let native = native.unwrap_or_else(|| {
        file.extension()
            .map(|e| e.eq_ignore_ascii_case("exe") || e.eq_ignore_ascii_case("appimage") || e.eq_ignore_ascii_case("sh"))
            .unwrap_or(false)
    });
    let title = title.filter(|t| !t.trim().is_empty()).unwrap_or_else(|| {
        scanner::clean_title(&file.file_stem().unwrap_or_default().to_string_lossy())
    });
    let game: Game = Game {
        id: library::new_id("g"),
        title,
        system_id,
        kind: if native { LaunchKind::Native } else { LaunchKind::Emulated },
        path,
        args: vec![],
        working_dir: None,
        emulator_override: None,
        cover: None,
        hero: None,
        logo: None,
        store: None,
        metadata: None,
        year: None,
        genre: None,
        description: None,
        favorite: false,
        playtime_secs: 0,
        last_played: None,
        added_at: library::now_unix(),
    };
    mutate_library(&app, |lib| lib.games.push(game.clone()))?;
    Ok(game)
}

/// Lance la synchro Steam en arrière-plan.
#[tauri::command]
fn sync_steam(state: State<AppState>) -> Result<(), String> {
    state.push(Job::SyncSteam)
}

/// Met à jour images et infos en arrière-plan : les jeux donnés, sinon ceux qui n'en ont
/// pas encore (tous si `force`).
#[tauri::command]
fn refresh_metadata(state: State<AppState>, game_ids: Option<Vec<String>>, force: bool) -> Result<(), String> {
    state.push(Job::RefreshMetadata { game_ids, force })
}

/// Choix manuel du jeu LaunchBox d'un jeu (appliqué en arrière-plan).
#[tauri::command]
fn set_launchbox_match(state: State<AppState>, game_id: String, lb_id: u64) -> Result<(), String> {
    state.push(Job::SetLaunchboxMatch { game_id, lb_id })
}

/// Retélécharge la base LaunchBox et reconstruit l'index en arrière-plan.
#[tauri::command]
fn update_launchbox_db(state: State<AppState>) -> Result<(), String> {
    state.push(Job::UpdateLaunchboxDb)
}

/// Recherche dans l'index LaunchBox local (`platform` : nom LaunchBox, ou toutes les
/// plateformes indexées). Vide si l'index n'existe pas encore.
#[tauri::command]
fn search_launchbox(state: State<AppState>, query: String, platform: Option<String>) -> Vec<LbGame> {
    let slot = state.loaded_index();
    let Some(idx) = slot.as_ref() else { return Vec::new() };
    launchbox::matching::search(&idx.games, &query, platform.as_deref())
        .into_iter()
        .take(20)
        .cloned()
        .collect()
}

/// Plateformes du catalogue LaunchBox, lues dans la base téléchargée (en mémoire ensuite).
/// Liste vide si la base n'est pas téléchargée ou illisible.
#[tauri::command]
fn list_lb_platforms(state: State<AppState>) -> Vec<LbPlatform> {
    let mut slot = lock(&state.lb_platforms);
    if slot.is_none() {
        let list = launchbox::index::platforms_or_empty(&state.launchbox_dir().join("Metadata.zip"));
        if list.is_empty() {
            return list;
        }
        *slot = Some(list);
    }
    slot.clone().unwrap_or_default()
}

/// Cherche des logos de console sur Wikimedia Commons.
#[tauri::command]
async fn search_logos(query: String) -> Result<Vec<logos::LogoCandidate>, String> {
    tauri::async_runtime::spawn_blocking(move || logos::search(&logos::client(), &query))
        .await
        .map_err(|e| format!("Recherche interrompue : {e}"))?
}

/// Enregistre le logo d'une console (URL `upload.wikimedia.org` ou fichier local) dans
/// `logos/<id>.<ext>` et renvoie son chemin. La bibliothèque n'est pas modifiée.
#[tauri::command]
async fn save_logo(
    state: State<'_, AppState>,
    system_id: String,
    source: String,
) -> Result<String, String> {
    let dir = state.data_dir().join("logos");
    tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        let ext = logos::ext_from(&source).ok_or("Extension du logo introuvable.")?;
        let bytes = if logos::is_http_url(&source) {
            if !logos::allowed_url(&source) {
                return Err("Seules les images de upload.wikimedia.org sont acceptées.".into());
            }
            logos::fetch_checked(&logos::client(), &source)?
        } else {
            let path = Path::new(&source);
            let size = std::fs::metadata(path).map_err(|e| format!("Fichier illisible : {e}"))?.len();
            if size > logos::MAX_LOGO_BYTES as u64 {
                return Err("Le logo dépasse 2 Mo.".into());
            }
            std::fs::read(path).map_err(|e| format!("Fichier illisible : {e}"))?
        };
        logos::validate_logo(&bytes, &ext)?;
        let stored = logos::store_logo(&bytes, &ext, &dir, &system_id)?;
        Ok(stored.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| format!("Enregistrement interrompu : {e}"))?
}

/// Supprime le fichier logo d'une console supprimée (sans effet s'il n'y en a pas).
#[tauri::command]
fn delete_logo(state: State<'_, AppState>, system_id: String) -> Result<(), String> {
    let dir = state.data_dir().join("logos");
    logos::delete_logo_files(&dir, &system_id)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct MetadataStatus {
    steam_root: Option<String>,
    /// Jeux Steam installés (manifeste présent).
    steam_games: u32,
    /// Date de la base LaunchBox téléchargée (secondes Unix).
    launchbox_date: Option<i64>,
    busy: bool,
}

/// État des bibliothèques et des images (lectures locales uniquement).
#[tauri::command]
fn metadata_status(state: State<AppState>) -> MetadataStatus {
    let manifests: Vec<String> = {
        let lib = lock(&state.library);
        lib.games
            .iter()
            .filter(|g| g.store.as_ref().is_some_and(|s| s.store == "steam"))
            .map(|g| g.path.clone())
            .collect()
    };
    let steam_games = manifests.iter().filter(|p| Path::new(p).is_file()).count() as u32;
    MetadataStatus {
        steam_root: stores::steam::steam_root().map(|p| p.to_string_lossy().into_owned()),
        steam_games,
        launchbox_date: metadata::mtime_secs(&state.launchbox_dir().join("Metadata.zip")),
        busy: state.pending.load(Ordering::SeqCst) > 0,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Première extension déclarée (exigence de la documentation) : une seconde instance
        // se ferme et ramène la fenêtre existante au premier plan, car deux instances sur le
        // même dossier de données s'écraseraient la bibliothèque.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let new_dir = app.path().app_data_dir()?;
            // Reprise des données de l'ancien identifiant (com.jhon.allone), avant tout chargement.
            // En cas d'échec, on travaille dans l'ancien dossier pour cette session (rien n'est
            // écrit dans le nouveau) : la reprise sera retentée au prochain lancement.
            let mut dir = new_dir.clone();
            if let Some(old_dir) = new_dir.parent().map(|p| p.join("com.jhon.allone")) {
                let migration = legacy::migrate_data_dir(&new_dir, &old_dir);
                match &migration {
                    Ok(true) => eprintln!("données reprises depuis {}", old_dir.display()),
                    Ok(false) => {}
                    Err(e) => eprintln!("reprise des anciennes données impossible : {e}"),
                }
                dir = legacy::choose_data_dir(&new_dir, &old_dir, &migration);
            }
            let path = dir.join("library.json");
            let library = Library::load_or_seed(&path);
            if !path.exists() {
                let _ = library.save(&path);
            }
            let jobs = worker::start(app.handle().clone());
            app.manage(AppState {
                library: Mutex::new(library),
                path,
                running: Mutex::new(HashSet::new()),
                jobs,
                pending: AtomicUsize::new(0),
                lb_index: Mutex::new(None),
                lb_platforms: Mutex::new(None),
            });
            // Au démarrage, en fond : synchro Steam puis images des jeux qui n'en ont pas.
            let state = app.state::<AppState>();
            state.push(Job::SyncSteam)?;
            state.push(Job::RefreshMetadata { game_ids: None, force: false })?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_library,
            save_library,
            check_files,
            preview_launch,
            launch_game,
            scan_system,
            add_game,
            sync_steam,
            refresh_metadata,
            set_launchbox_match,
            update_launchbox_db,
            search_launchbox,
            list_lb_platforms,
            search_logos,
            save_logo,
            delete_logo,
            metadata_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
