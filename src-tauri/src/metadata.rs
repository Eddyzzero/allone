//! Images et infos des jeux : choix des jeux à traiter, récupération (Steam, LaunchBox)
//! et application à un jeu. Aucune fonction ici ne touche à la bibliothèque partagée.

use crate::launchbox::index::{LbGame, LbIndex};
use crate::launchbox::matching::{fr_genre, match_game};
use crate::library::{Game, Library, MetaRef, MetaSource};
use crate::media::{self, ImageKind};
use crate::stores::steam_web::{self, SteamItem, SteamLocalImages, StoreDetails};
use reqwest::blocking::Client;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const LB_IMAGES: &str = "https://images.launchbox-app.com/";

/// Ce qu'une récupération a obtenu pour un jeu (images déjà enregistrées sur le disque).
#[derive(Debug, Clone, PartialEq)]
pub struct MetaFields {
    pub cover: Option<PathBuf>,
    pub hero: Option<PathBuf>,
    pub logo: Option<PathBuf>,
    pub description: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u16>,
    /// `None` : résultat partiel (boutique injoignable) ; le jeu reste sans `metadata`
    /// et sera retenté.
    pub meta: Option<MetaRef>,
}

/// Applique `f` au jeu. Le titre n'est jamais touché.
/// - `replace` (choix manuel) : tout est remplacé, un champ absent de `f` est vidé.
/// - sinon : images remplacées quand `f` en a ; description, genre et année seulement s'ils
///   sont vides. Si `f` vient d'une autre correspondance que celle du jeu, les images que la
///   nouvelle n'a pas sont vidées (elles appartenaient à un autre jeu).
///
/// Renvoie vrai si un champ du jeu a réellement changé (sinon, rien à enregistrer).
pub fn apply(game: &mut Game, f: MetaFields, replace: bool) -> bool {
    let before = snapshot(game);
    apply_fields(game, f, replace);
    snapshot(game) != before
}

type Snapshot = ([Option<String>; 3], Option<String>, Option<String>, Option<u16>, Option<MetaRef>);

/// Champs qu'une récupération peut modifier.
fn snapshot(game: &Game) -> Snapshot {
    (images_of(game), game.description.clone(), game.genre.clone(), game.year, game.metadata.clone())
}

fn apply_fields(game: &mut Game, f: MetaFields, replace: bool) {
    let rematch = match (&game.metadata, &f.meta) {
        (Some(old), Some(new)) => old.source != new.source || old.id != new.id,
        _ => false,
    };
    let clear_images = replace || rematch;
    for (slot, new) in [(&mut game.cover, f.cover), (&mut game.hero, f.hero), (&mut game.logo, f.logo)] {
        match new {
            Some(p) => *slot = Some(p.to_string_lossy().into_owned()),
            None if clear_images => *slot = None,
            None => {}
        }
    }
    if replace {
        game.description = f.description;
        game.genre = f.genre;
        game.year = f.year;
    } else {
        let empty = |s: &Option<String>| s.as_deref().is_none_or(|s| s.trim().is_empty());
        if f.description.is_some() && empty(&game.description) {
            game.description = f.description;
        }
        if f.genre.is_some() && empty(&game.genre) {
            game.genre = f.genre;
        }
        if f.year.is_some() && game.year.is_none() {
            game.year = f.year;
        }
    }
    if let Some(meta) = f.meta {
        game.metadata = Some(meta);
    }
}

/// Images du jeu (jaquette, fond, logo).
pub fn images_of(game: &Game) -> [Option<String>; 3] {
    [game.cover.clone(), game.hero.clone(), game.logo.clone()]
}

/// Images de `before` que le jeu ne référence plus et qui sont dans son dossier `dir`
/// (une image choisie ailleurs par l'utilisateur n'est jamais supprimée).
pub fn stale_images(before: [Option<String>; 3], game: &Game, dir: &Path) -> Vec<PathBuf> {
    let now = images_of(game);
    before
        .into_iter()
        .flatten()
        .filter(|p| !now.iter().flatten().any(|n| n == p))
        .map(PathBuf::from)
        .filter(|p| p.starts_with(dir))
        .collect()
}

/// Déplace les images obtenues (dossier provisoire) dans `dir`, en remplaçant les anciennes.
pub fn commit_images(f: &mut MetaFields, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("Création du dossier impossible : {e}"))?;
    for slot in [&mut f.cover, &mut f.hero, &mut f.logo] {
        let Some(src) = slot.take() else { continue };
        let dest = dir.join(src.file_name().ok_or("Nom d'image invalide")?);
        std::fs::rename(&src, &dest).map_err(|e| format!("Enregistrement de l'image impossible : {e}"))?;
        *slot = Some(dest);
    }
    Ok(())
}

/// Jeux à traiter : ceux de `game_ids` qui existent ; sinon les jeux sans `metadata`
/// (tous si `force`), plus ceux dont une image locale (couverture, bandeau, logo) pointe
/// vers un fichier disparu. Un jeu dont la récupération a échoué n'a pas reçu de `metadata`
/// et revient donc au prochain appel.
pub fn plan_refresh(lib: &Library, game_ids: Option<&[String]>, force: bool) -> Vec<String> {
    plan_refresh_with(lib, game_ids, force, &|p| Path::new(p).exists())
}

/// Chemin d'image local (ni adresse web, ni chemin web commençant par « / »).
fn is_local_image(p: &str) -> bool {
    !p.is_empty() && !p.starts_with('/') && !p.starts_with("http://") && !p.starts_with("https://")
}

fn has_missing_image(game: &Game, exists: &dyn Fn(&str) -> bool) -> bool {
    [&game.cover, &game.hero, &game.logo]
        .into_iter()
        .flatten()
        .any(|p| is_local_image(p) && !exists(p))
}

fn plan_refresh_with(
    lib: &Library,
    game_ids: Option<&[String]>,
    force: bool,
    exists: &dyn Fn(&str) -> bool,
) -> Vec<String> {
    match game_ids {
        Some(ids) => ids.iter().filter(|id| lib.game(id).is_some()).cloned().collect(),
        None => lib
            .games
            .iter()
            .filter(|g| force || g.metadata.is_none() || has_missing_image(g, exists))
            .map(|g| g.id.clone())
            .collect(),
    }
}

/// D'où viennent les images et infos d'un jeu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Steam(u32),
    Launchbox(String),
}

/// Source d'un jeu : Steam pour un jeu Steam, LaunchBox si son système a une plateforme
/// LaunchBox, sinon aucune.
pub fn source_of(lib: &Library, game: &Game) -> Option<Source> {
    match &game.store {
        Some(s) if s.store == "steam" => s.id.parse().ok().map(Source::Steam),
        Some(_) => None,
        None => lib.system(&game.system_id).and_then(|s| s.lb_platform.clone()).map(Source::Launchbox),
    }
}

/// Plateformes LaunchBox des systèmes de la bibliothèque, triées et sans doublon.
pub fn lb_platforms(lib: &Library) -> Vec<String> {
    let mut out: Vec<String> = lib.systems.iter().filter_map(|s| s.lb_platform.clone()).collect();
    out.sort();
    out.dedup();
    out
}

/// L'index correspond-il au zip (même date) et aux plateformes actuelles ?
pub fn index_is_current(idx: &LbIndex, zip_mtime: i64, platforms: &[String]) -> bool {
    idx.built_from == zip_mtime && idx.platforms == platforms
}

/// Date de modification d'un fichier (secondes Unix).
pub fn mtime_secs(path: &Path) -> Option<i64> {
    let t = std::fs::metadata(path).and_then(|m| m.modified()).ok()?;
    Some(t.duration_since(UNIX_EPOCH).ok()?.as_secs() as i64)
}

/// Jeu LaunchBox d'un jeu : la correspondance verrouillée si elle existe (sans recherche,
/// même sur une autre plateforme), sinon la correspondance par titre puis nom de fichier.
/// Renvoie aussi si elle est verrouillée.
pub fn find_launchbox<'a>(games: &'a [LbGame], game: &Game, platform: &str) -> Option<(&'a LbGame, bool)> {
    match &game.metadata {
        Some(m) if m.locked && m.source == MetaSource::Launchbox => {
            let id: u64 = m.id.parse().ok()?;
            games.iter().find(|g| g.id == id).map(|g| (g, true))
        }
        _ => match_game(games, platform, &game.title, Some(&game.path)).map(|g| (g, false)),
    }
}

/// Une image à obtenir : d'abord `url`, sinon une copie du fichier `local`.
struct ImageSrc {
    kind: ImageKind,
    url: Option<String>,
    local: Option<PathBuf>,
}

fn obtain(client: &Client, dir: &Path, src: ImageSrc) -> Option<PathBuf> {
    if let Some(url) = &src.url {
        match media::fetch(client, url).and_then(|b| media::store_image(&b, dir, src.kind)) {
            Ok(p) => return Some(p),
            Err(e) => eprintln!("Image non récupérée : {e}"),
        }
    }
    let local = src.local?;
    let stored = std::fs::read(&local)
        .map_err(|e| format!("Lecture de {} impossible : {e}", local.display()))
        .and_then(|b| media::store_image(&b, dir, src.kind));
    match stored {
        Ok(p) => Some(p),
        Err(e) => {
            eprintln!("Image locale non copiée : {e}");
            None
        }
    }
}

/// Obtient les trois images (jaquette, fond, logo) en parallèle dans `dir`.
fn store_images(client: &Client, dir: &Path, srcs: [ImageSrc; 3]) -> [Option<PathBuf>; 3] {
    std::thread::scope(|s| {
        let handles = srcs.map(|src| s.spawn(move || obtain(client, dir, src)));
        handles.map(|h| h.join().ok().flatten())
    })
}

/// Assemble le résultat Steam ; `None` si rien n'a pu être obtenu. `metadata` n'est posé que si
/// la boutique a répondu pour ce jeu (`reached`) : sinon, images du cache local seulement, et le
/// jeu sera retenté avec les images et la fiche de la boutique.
pub fn steam_fields(
    app_id: u32,
    images: [Option<PathBuf>; 3],
    details: Option<StoreDetails>,
    reached: bool,
) -> Option<MetaFields> {
    let [cover, hero, logo] = images;
    if cover.is_none() && hero.is_none() && logo.is_none() && details.is_none() {
        return None;
    }
    let (description, genre, year) = details.map_or((None, None, None), |d| (d.description, d.genre, d.year));
    Some(MetaFields {
        cover,
        hero,
        logo,
        description,
        genre,
        year,
        meta: reached.then(|| MetaRef { source: MetaSource::Steam, id: app_id.to_string(), locked: false }),
    })
}

/// Images et infos d'un jeu Steam : URL de la boutique (`item`), sinon cache local de Steam
/// image par image ; fiche boutique en parallèle (4 téléchargements au plus). `item` absent :
/// la boutique n'a pas répondu pour ce jeu.
pub fn fetch_steam(
    client: &Client,
    app_id: u32,
    item: Option<&SteamItem>,
    local: SteamLocalImages,
    dir: &Path,
) -> Option<MetaFields> {
    let url = |pick: fn(&SteamItem) -> &Option<String>| item.and_then(|i| pick(i).clone());
    let srcs = [
        ImageSrc { kind: ImageKind::Cover, url: url(|i| &i.cover), local: local.cover },
        ImageSrc { kind: ImageKind::Hero, url: url(|i| &i.hero), local: local.hero },
        ImageSrc { kind: ImageKind::Logo, url: url(|i| &i.logo), local: local.logo },
    ];
    let (images, details) = std::thread::scope(|s| {
        let details = s.spawn(|| steam_web::app_details(client, app_id));
        let images = store_images(client, dir, srcs);
        (images, details.join())
    });
    let details = match details {
        Ok(Ok(d)) => d,
        Ok(Err(e)) => {
            eprintln!("Fiche Steam {app_id} indisponible : {e}");
            None
        }
        Err(_) => None,
    };
    steam_fields(app_id, images, details, item.is_some())
}

/// Assemble le résultat LaunchBox ; `None` si le jeu a des images mais qu'aucune n'a pu
/// être téléchargée (problème de réseau : le jeu sera retenté).
pub fn launchbox_fields(lb: &LbGame, locked: bool, images: [Option<PathBuf>; 3]) -> Option<MetaFields> {
    let [cover, hero, logo] = images;
    let listed = lb.cover.is_some() || lb.hero.is_some() || lb.logo.is_some();
    if listed && cover.is_none() && hero.is_none() && logo.is_none() {
        return None;
    }
    Some(MetaFields {
        cover,
        hero,
        logo,
        description: lb.overview.clone(),
        genre: lb.genres.first().map(|g| fr_genre(g)),
        year: lb.year,
        meta: Some(MetaRef { source: MetaSource::Launchbox, id: lb.id.to_string(), locked }),
    })
}

/// Une image prévue par LaunchBox manque-t-elle (téléchargement échoué) ?
pub fn missing_listed_images(lb: &LbGame, f: &MetaFields) -> bool {
    (lb.cover.is_some() && f.cover.is_none())
        || (lb.hero.is_some() && f.hero.is_none())
        || (lb.logo.is_some() && f.logo.is_none())
}

/// Images et infos d'un jeu LaunchBox (images téléchargées en parallèle).
pub fn fetch_launchbox(client: &Client, lb: &LbGame, locked: bool, dir: &Path) -> Option<MetaFields> {
    let url = |file: &Option<String>| file.as_ref().map(|f| format!("{LB_IMAGES}{f}"));
    let srcs = [
        ImageSrc { kind: ImageKind::Cover, url: url(&lb.cover), local: None },
        ImageSrc { kind: ImageKind::Hero, url: url(&lb.hero), local: None },
        ImageSrc { kind: ImageKind::Logo, url: url(&lb.logo), local: None },
    ];
    let images = store_images(client, dir, srcs);
    launchbox_fields(lb, locked, images)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::{CURRENT_VERSION, System};
    use serde_json::json;

    fn game(id: &str, system: &str) -> Game {
        serde_json::from_value(json!({
            "id": id, "title": format!("Titre {id}"), "systemId": system, "kind": "emulated", "path": "x.iso"
        }))
        .unwrap()
    }

    fn steam_game(id: &str, app_id: u32) -> Game {
        serde_json::from_value(json!({
            "id": id, "title": "Jeu Steam", "systemId": "steam", "kind": "store", "path": "m.acf",
            "store": { "store": "steam", "id": app_id.to_string() }
        }))
        .unwrap()
    }

    fn lib(games: Vec<Game>) -> Library {
        Library { version: CURRENT_VERSION, revision: 0, emulators: vec![], systems: vec![], games }
    }

    fn system(id: &str, platform: Option<&str>) -> System {
        System {
            id: id.into(),
            name: id.into(),
            short_name: id.into(),
            color: "#000".into(),
            year: None,
            emulator_id: None,
            rom_dirs: vec![],
            extensions: vec![],
            lb_platform: platform.map(String::from),
            logo: None,
        }
    }

    fn lb_meta(id: &str, locked: bool) -> MetaRef {
        MetaRef { source: MetaSource::Launchbox, id: id.into(), locked }
    }

    fn fields() -> MetaFields {
        MetaFields {
            cover: Some(PathBuf::from("/m/g/cover.jpg")),
            hero: None,
            logo: Some(PathBuf::from("/m/g/logo.png")),
            description: Some("Un jeu.".into()),
            genre: Some("Action".into()),
            year: Some(2005),
            meta: Some(lb_meta("26", false)),
        }
    }

    fn lb_game(id: u64, name: &str, platform: &str) -> LbGame {
        LbGame {
            id,
            name: name.into(),
            alt_names: vec![],
            platform: platform.into(),
            year: Some(2005),
            overview: Some("Kratos.".into()),
            genres: vec!["Racing".into(), "Action".into()],
            developer: None,
            publisher: None,
            cover: Some("c.jpg".into()),
            hero: None,
            logo: None,
        }
    }

    #[test]
    fn apply_fills_only_empty_fields() {
        let mut g = game("g", "ps2");
        g.genre = Some("Course".into());
        g.hero = Some("/old/hero.jpg".into());
        apply(&mut g, fields(), false);
        assert_eq!(g.genre.as_deref(), Some("Course"));
        assert_eq!(g.description.as_deref(), Some("Un jeu."));
        assert_eq!(g.year, Some(2005));
        assert_eq!(g.title, "Titre g");
        assert_eq!(g.cover.as_deref(), Some(PathBuf::from("/m/g/cover.jpg").to_string_lossy().as_ref()));
        assert_eq!(g.hero.as_deref(), Some("/old/hero.jpg"));
        assert_eq!(g.metadata, Some(lb_meta("26", false)));
    }

    #[test]
    fn apply_replace_overwrites_all_but_title() {
        let mut g = game("g", "ps2");
        g.genre = Some("Course".into());
        g.description = Some("Ancienne".into());
        g.year = Some(1999);
        g.cover = Some("/old/cover.jpg".into());
        g.metadata = Some(lb_meta("26", false));
        let f = MetaFields { meta: Some(lb_meta("26", true)), ..fields() };
        apply(&mut g, f, true);
        assert_eq!(g.genre.as_deref(), Some("Action"));
        assert_eq!(g.description.as_deref(), Some("Un jeu."));
        assert_eq!(g.year, Some(2005));
        assert_ne!(g.cover.as_deref(), Some("/old/cover.jpg"));
        assert_eq!(g.title, "Titre g");
        assert_eq!(g.metadata, Some(lb_meta("26", true)));

        // Ce que le nouveau jeu n'a pas est vidé : rien ne reste de l'ancienne correspondance.
        let mut g = game("g", "ps2");
        g.hero = Some("/old/hero.jpg".into());
        g.logo = Some("/old/logo.png".into());
        g.description = Some("Ancienne".into());
        g.genre = Some("Course".into());
        g.year = Some(1999);
        g.metadata = Some(lb_meta("26", false));
        let f = MetaFields { cover: None, hero: None, logo: None, description: None, genre: None, year: None, ..fields() };
        apply(&mut g, f, true);
        assert_eq!((g.cover, g.hero, g.logo), (None, None, None));
        assert_eq!((g.description, g.genre, g.year), (None, None, None));
        assert_eq!(g.title, "Titre g");
    }

    #[test]
    fn apply_reports_no_change_on_identical_result() {
        // Démarrage hors ligne répété : mêmes images du cache local, pas de `metadata`.
        let mut g = steam_game("s", 42);
        let offline = || steam_fields(42, [Some(PathBuf::from("/m/s/cover.jpg")), None, None], None, false).unwrap();
        assert!(apply(&mut g, offline(), false));
        assert!(!apply(&mut g, offline(), false));

        // Même résultat complet appliqué deux fois : aucun changement la seconde fois.
        let mut g = game("g", "ps2");
        assert!(apply(&mut g, fields(), false));
        assert!(!apply(&mut g, fields(), false));
        assert!(!apply(&mut g, fields(), true));
        assert!(apply(&mut g, MetaFields { year: Some(2006), ..fields() }, true));
    }

    #[test]
    fn apply_with_other_match_clears_missing_images_only() {
        let mut g = game("g", "ps2");
        g.hero = Some("/old/hero.jpg".into());
        g.description = Some("Ancienne".into());
        g.metadata = Some(lb_meta("99", false));
        apply(&mut g, fields(), false);
        assert_eq!(g.hero, None);
        assert!(g.cover.is_some() && g.logo.is_some());
        assert_eq!(g.description.as_deref(), Some("Ancienne"));
        assert_eq!(g.metadata, Some(lb_meta("26", false)));

        // Même correspondance : une image manquante (téléchargement raté) garde l'ancienne.
        let mut g = game("g", "ps2");
        g.hero = Some("/old/hero.jpg".into());
        g.metadata = Some(lb_meta("26", false));
        apply(&mut g, fields(), false);
        assert_eq!(g.hero.as_deref(), Some("/old/hero.jpg"));
    }

    #[test]
    fn stale_images_only_inside_game_media_dir() {
        let dir = PathBuf::from("/data/media/g");
        let inside = |n: &str| Some(dir.join(n).to_string_lossy().into_owned());
        let mut g = game("g", "ps2");
        let before = [inside("cover.jpg"), inside("hero.jpg"), Some("/ailleurs/logo.png".into())];
        g.cover = inside("cover.jpg");
        assert_eq!(stale_images(before, &g, &dir), vec![dir.join("hero.jpg")]);
    }

    #[test]
    fn commit_images_moves_into_media_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let staging = tmp.path().join("g").join(".en-cours");
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::write(staging.join("cover.jpg"), b"nouveau").unwrap();
        let dir = tmp.path().join("g");
        std::fs::write(dir.join("cover.jpg"), b"ancien").unwrap();
        let mut f = MetaFields { cover: Some(staging.join("cover.jpg")), logo: None, ..fields() };
        commit_images(&mut f, &dir).unwrap();
        assert_eq!(f.cover, Some(dir.join("cover.jpg")));
        assert_eq!(std::fs::read(dir.join("cover.jpg")).unwrap(), b"nouveau");
        assert!(!staging.join("cover.jpg").exists());
    }

    #[test]
    fn offline_job_failure_leaves_game_untouched() {
        // Image réellement présente sur disque (sinon le jeu serait replanifié).
        let tmp = tempfile::tempdir().unwrap();
        let cover = tmp.path().join("c.jpg");
        std::fs::write(&cover, b"x").unwrap();
        let mut done = game("done", "ps2");
        done.metadata = Some(lb_meta("26", false));
        let mut l = lib(vec![steam_game("s", 42), done]);
        assert_eq!(plan_refresh(&l, None, false), vec!["s".to_string()]);

        // Hors ligne et sans cache local : rien d'obtenu, donc rien à appliquer.
        assert_eq!(steam_fields(42, [None, None, None], None, false), None);

        // Hors ligne avec le cache local de Steam : l'image est appliquée, mais pas `metadata`,
        // donc le jeu est retenté (images et fiche de la boutique) au prochain passage.
        let f = steam_fields(42, [Some(cover.clone()), None, None], None, false).unwrap();
        assert_eq!(f.meta, None);
        apply(&mut l.games[0], f, false);
        assert!(l.games[0].cover.is_some());
        assert_eq!(l.games[0].metadata, None);
        assert_eq!(plan_refresh(&l, None, false), vec!["s".to_string()]);

        // Boutique jointe : le jeu reçoit `metadata` et sort de la liste.
        let f = steam_fields(42, [Some(cover.clone()), None, None], None, true).unwrap();
        assert_eq!(f.meta, Some(MetaRef { source: MetaSource::Steam, id: "42".into(), locked: false }));
        apply(&mut l.games[0], f, false);
        assert!(plan_refresh(&l, None, false).is_empty());
        assert_eq!(plan_refresh(&l, None, true).len(), 2);
        let ids = ["done".to_string(), "absent".to_string()];
        assert_eq!(plan_refresh(&l, Some(&ids), false), vec!["done".to_string()]);
    }

    #[test]
    fn plan_refresh_includes_games_with_missing_local_images() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("cover.jpg");
        let mut g = steam_game("s", 42);
        g.metadata = Some(MetaRef { source: MetaSource::Steam, id: "42".into(), locked: false });
        g.cover = Some(file.to_string_lossy().into_owned());
        let l = lib(vec![g]);
        assert_eq!(plan_refresh(&l, None, false), vec!["s".to_string()]);
        std::fs::write(&file, b"x").unwrap();
        assert!(plan_refresh(&l, None, false).is_empty());
        // Chemins web ou adresses : jamais considérés comme manquants.
        let mut l = l;
        l.games[0].hero = Some("/images/hero.jpg".into());
        l.games[0].logo = Some("https://exemple.test/logo.png".into());
        assert!(plan_refresh(&l, None, false).is_empty());
    }

    #[test]
    fn source_of_distinguishes_steam_launchbox_and_unknown() {
        let mut l = lib(vec![]);
        l.systems = vec![system("ps2", Some("Sony Playstation 2")), system("snes", None)];
        assert_eq!(source_of(&l, &steam_game("s", 42)), Some(Source::Steam(42)));
        assert_eq!(source_of(&l, &game("g", "ps2")), Some(Source::Launchbox("Sony Playstation 2".into())));
        assert_eq!(source_of(&l, &game("g", "snes")), None);
        assert_eq!(source_of(&l, &game("g", "inconnu")), None);
    }

    #[test]
    fn lb_platforms_sorted_and_deduped() {
        let sys = |id: &str| {
            let platform = match id {
                "psp" => Some("Sony PSP"),
                "ps2" => Some("Sony Playstation 2"),
                _ => None,
            };
            system(id, platform)
        };
        let mut l = lib(vec![]);
        l.systems = vec![sys("psp"), sys("steam"), sys("ps2"), sys("psp")];
        let platforms = lb_platforms(&l);
        assert_eq!(platforms, vec!["Sony PSP".to_string(), "Sony Playstation 2".to_string()]);

        let idx = LbIndex { built_from: 10, platforms: platforms.clone(), games: vec![] };
        assert!(index_is_current(&idx, 10, &platforms));
        assert!(!index_is_current(&idx, 11, &platforms));
        assert!(!index_is_current(&idx, 10, &platforms[..1]));
    }

    #[test]
    fn lb_platforms_reads_system_field() {
        let mut l = lib(vec![]);
        l.systems = vec![system("gc", Some("Nintendo GameCube"))];
        assert_eq!(lb_platforms(&l), vec!["Nintendo GameCube".to_string()]);
    }

    #[test]
    fn locked_match_reuses_id_without_matching() {
        let games = vec![lb_game(26, "God of War", "Sony Playstation 2"), lb_game(500, "Autre", "Sony PSP")];
        let mut g = game("g", "ps2");
        g.title = "God of War".into();
        assert_eq!(find_launchbox(&games, &g, "Sony Playstation 2").map(|(l, k)| (l.id, k)), Some((26, false)));

        g.metadata = Some(lb_meta("500", true));
        assert_eq!(find_launchbox(&games, &g, "Sony Playstation 2").map(|(l, k)| (l.id, k)), Some((500, true)));

        // Verrouillé sur un id absent de l'index : pas de recherche par titre.
        g.metadata = Some(lb_meta("999", true));
        assert!(find_launchbox(&games, &g, "Sony Playstation 2").is_none());
    }

    #[test]
    fn launchbox_fields_translate_genre_and_detect_failure() {
        let lb = lb_game(26, "God of War", "Sony Playstation 2");
        assert_eq!(launchbox_fields(&lb, false, [None, None, None]), None);
        let f = launchbox_fields(&lb, true, [Some(PathBuf::from("c.jpg")), None, None]).unwrap();
        assert_eq!(f.genre.as_deref(), Some("Course"));
        assert_eq!(f.description.as_deref(), Some("Kratos."));
        assert_eq!(f.meta, Some(lb_meta("26", true)));
        assert!(!missing_listed_images(&lb, &f));
        let partial = MetaFields { cover: None, ..f.clone() };
        assert!(missing_listed_images(&lb, &partial));

        let bare = LbGame { cover: None, ..lb };
        assert!(launchbox_fields(&bare, false, [None, None, None]).is_some());
    }

    #[test]
    fn image_falls_back_to_local_cache_when_download_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let local = tmp.path().join("library_600x900.jpg");
        let img = image::RgbImage::from_pixel(60, 90, image::Rgb([10, 20, 30]));
        img.save(&local).unwrap();
        let dir = tmp.path().join("media").join("g");
        let client = media::http_client();
        let srcs = [
            ImageSrc { kind: ImageKind::Cover, url: Some("adresse-invalide://x".into()), local: Some(local) },
            ImageSrc { kind: ImageKind::Hero, url: None, local: None },
            ImageSrc { kind: ImageKind::Logo, url: Some("adresse-invalide://y".into()), local: None },
        ];
        let [cover, hero, logo] = store_images(&client, &dir, srcs);
        assert_eq!(cover, Some(dir.join("cover.jpg")));
        assert!(dir.join("cover.jpg").is_file());
        assert_eq!((hero, logo), (None, None));
    }
}
