//! Détection de Steam et de ses jeux installés.

use crate::library::{self, Game, LaunchKind, Library, StoreRef, System};
use crate::vdf::{self, Vdf};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct SteamApp {
    pub app_id: u32,
    pub name: String,
    pub manifest_path: PathBuf,
    pub install_dir: PathBuf,
}

/// Dossier racine de Steam, s'il existe.
pub fn steam_root() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        use winreg::{RegKey, enums::HKEY_CURRENT_USER};
        let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Software\\Valve\\Steam").ok()?;
        let path: String = key.get_value("SteamPath").ok()?;
        // Le registre stocke des barres obliques : on normalise en chemin Windows.
        let root = PathBuf::from(path.replace('/', "\\"));
        root.is_dir().then_some(root)
    }
    #[cfg(not(windows))]
    {
        let home = PathBuf::from(std::env::var_os("HOME")?);
        [".steam/steam", ".local/share/Steam", ".var/app/com.valvesoftware.Steam/.local/share/Steam"]
            .iter()
            .map(|p| home.join(p))
            .find(|p| p.is_dir())
    }
}

fn read_vdf(path: &Path) -> Option<Vdf> {
    vdf::parse(&std::fs::read_to_string(path).ok()?).ok()
}

/// Clé de comparaison d'un dossier : chemin canonique (sinon brut), normalisé
/// (insensible à la casse et aux séparateurs sous Windows, sans séparateur final).
fn dir_key(p: &Path) -> String {
    let real = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let s = real.to_string_lossy().into_owned();
    #[cfg(windows)]
    let s = s.trim_start_matches(r"\\?\").replace('/', "\\").to_lowercase();
    s.trim_end_matches(['/', '\\']).to_string()
}

/// Bibliothèques Steam : la racine, puis celles de `libraryfolders.vdf`, sans doublons.
/// Pour un même dossier, la graphie de `libraryfolders.vdf` (vraie casse) remplace celle du registre.
pub fn library_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![root.to_path_buf()];
    let mut keys = vec![dir_key(root)];
    if let Some(v) = read_vdf(&root.join("steamapps").join("libraryfolders.vdf")) {
        if let Some(folders) = v.get("libraryfolders") {
            for (_, entry) in folders.entries() {
                if let Some(p) = entry.get("path").and_then(Vdf::str) {
                    let p = PathBuf::from(p);
                    let key = dir_key(&p);
                    match keys.iter().position(|k| *k == key) {
                        Some(0) if dirs[0] == root => dirs[0] = p,
                        Some(_) => {}
                        None => {
                            keys.push(key);
                            dirs.push(p);
                        }
                    }
                }
            }
        }
    }
    dirs
}

fn read_manifest(lib: &Path, path: &Path) -> Option<SteamApp> {
    let v = read_vdf(path)?;
    let state = v.get("AppState")?;
    let app_id = state.get("appid")?.str()?.trim().parse().ok()?;
    let name = state.get("name")?.str()?.to_string();
    let install_dir = lib.join("steamapps").join("common").join(state.get("installdir")?.str()?);
    Some(SteamApp { app_id, name, manifest_path: path.to_path_buf(), install_dir })
}

/// Jeux installés de toutes les bibliothèques ; un manifeste illisible est ignoré.
pub fn installed_apps(root: &Path) -> Vec<SteamApp> {
    let mut apps = Vec::new();
    for lib in library_dirs(root) {
        let Ok(entries) = std::fs::read_dir(lib.join("steamapps")) else { continue };
        let mut paths: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("appmanifest_") && n.ends_with(".acf"))
            })
            .collect();
        paths.sort();
        apps.extend(paths.iter().filter_map(|p| read_manifest(&lib, p)));
    }
    apps
}

/// Temps de jeu (secondes) et dernière partie, depuis le `localconfig.vdf` le plus récent.
pub fn playtimes(root: &Path) -> HashMap<u32, (u64, Option<i64>)> {
    let mut out = HashMap::new();
    let Ok(users) = std::fs::read_dir(root.join("userdata")) else { return out };
    let newest = users
        .flatten()
        .map(|u| u.path().join("config").join("localconfig.vdf"))
        .filter_map(|p| Some((std::fs::metadata(&p).ok()?.modified().ok()?, p)))
        .max_by_key(|(t, _)| *t);
    let Some((_, path)) = newest else { return out };
    let Some(v) = read_vdf(&path) else { return out };
    let Some(apps) = v.path(&["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"]) else {
        return out;
    };
    for (id, app) in apps.entries() {
        let Ok(id) = id.parse::<u32>() else { continue };
        // Sans `Playtime`, le jeu compte 0 seconde.
        let minutes: u64 = app.get("Playtime").and_then(Vdf::str).and_then(|s| s.parse().ok()).unwrap_or(0);
        let last = app.get("LastPlayed").and_then(Vdf::str).and_then(|s| s.parse().ok());
        out.insert(id, (minutes * 60, last));
    }
    out
}

/// Entrées techniques (redistribuables, Proton…) à ne pas importer comme jeux.
pub fn offline_excluded(app: &SteamApp) -> bool {
    if app.app_id == 228980 {
        return true;
    }
    let name = app.name.to_lowercase();
    ["redistributable", "proton", "steam linux runtime", "steamworks"].iter().any(|k| name.contains(k))
}

/// Type d'une entrée du catalogue Steam, quand il est connu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreType {
    Game,
    Other,
    Unknown,
}

/// Résultat d'une synchro : jeux créés (ids) et nombre de modifications réelles
/// sur l'existant (jeux dont path/temps/dernière partie ont changé, système créé).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncReport {
    pub added: Vec<String>,
    pub updated: usize,
}

fn is_steam_game(g: &Game, app_id: u32) -> bool {
    g.store.as_ref().is_some_and(|s| s.store == "steam" && s.id == app_id.to_string())
}

/// Intègre les jeux Steam installés dans la bibliothèque : ajoute les nouveaux,
/// met à jour temps de jeu et dernière partie des existants. Un jeu désinstallé
/// n'est jamais supprimé.
pub fn sync_into(
    lib: &mut Library,
    apps: &[SteamApp],
    playtimes: &HashMap<u32, (u64, Option<i64>)>,
    store_types: Option<&HashMap<u32, StoreType>>,
) -> SyncReport {
    let mut report = SyncReport { added: vec![], updated: 0 };
    let kept: Vec<&SteamApp> = apps
        .iter()
        .filter(|a| {
            match store_types.and_then(|t| t.get(&a.app_id)).copied().unwrap_or(StoreType::Unknown) {
                StoreType::Game => true,
                StoreType::Other => false,
                StoreType::Unknown => !offline_excluded(a),
            }
        })
        .collect();
    if kept.is_empty() {
        return report;
    }
    if lib.system("steam").is_none() {
        report.updated += 1;
        lib.systems.push(System {
            id: "steam".into(),
            name: "Steam".into(),
            short_name: "Steam".into(),
            color: "#66C0F4".into(),
            year: None,
            emulator_id: None,
            rom_dirs: vec![],
            extensions: vec![],
            lb_platform: None,
            logo: Some("/systems/steam.svg".into()),
        });
    }
    for app in kept {
        let path = app.manifest_path.to_string_lossy().into_owned();
        let played = playtimes.get(&app.app_id);
        if let Some(g) = lib.games.iter_mut().find(|g| is_steam_game(g, app.app_id)) {
            let before = (g.path.clone(), g.playtime_secs, g.last_played);
            g.path = path;
            if let Some(&(secs, last)) = played {
                g.playtime_secs = secs;
                g.last_played = g.last_played.max(last);
            }
            if before != (g.path.clone(), g.playtime_secs, g.last_played) {
                report.updated += 1;
            }
            continue;
        }
        let (secs, last) = played.copied().unwrap_or((0, None));
        let id = library::new_id("steam");
        lib.games.push(Game {
            id: id.clone(),
            title: app.name.clone(),
            system_id: "steam".into(),
            kind: LaunchKind::Store,
            path,
            args: vec![],
            working_dir: None,
            emulator_override: None,
            cover: None,
            hero: None,
            logo: None,
            store: Some(StoreRef { store: "steam".into(), id: app.app_id.to_string() }),
            metadata: None,
            year: None,
            genre: None,
            description: None,
            favorite: false,
            playtime_secs: secs,
            playnite_secs: 0,
            last_played: last,
            added_at: library::now_unix(),
        });
        report.added.push(id);
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vdf::fixtures::{LOCALCONFIG, MANIFEST};
    use std::fs;
    use std::time::{Duration, SystemTime};

    fn manifest(id: u32, name: &str, dir: &str) -> String {
        format!("\"AppState\"\n{{\n\t\"appid\"\t\t\"{id}\"\n\t\"name\"\t\t\"{name}\"\n\t\"installdir\"\t\t\"{dir}\"\n}}\n")
    }

    fn vdf_path(p: &Path) -> String {
        p.to_string_lossy().replace('\\', "\\\\")
    }

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn set_mtime(path: &Path, secs: u64) {
        let f = fs::OpenOptions::new().write(true).open(path).unwrap();
        f.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(secs)).unwrap();
    }

    /// Faux Steam : racine + bibliothèque secondaire, trois manifestes, deux localconfig.
    fn fake_steam() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Steam");
        let lib2 = tmp.path().join("SteamLibrary");
        let folders = format!(
            "\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n\t\"1\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n}}\n",
            vdf_path(&root),
            vdf_path(&lib2)
        );
        write(&root.join("steamapps/libraryfolders.vdf"), &folders);
        write(&root.join("steamapps/appmanifest_1426210.acf"), MANIFEST);
        write(
            &root.join("steamapps/appmanifest_228980.acf"),
            &manifest(228980, "Steamworks Common Redistributables", "Steamworks Shared"),
        );
        write(&lib2.join("steamapps/appmanifest_620.acf"), &manifest(620, "Portal 2", "Portal 2"));
        write(&lib2.join("steamapps/appmanifest_999.acf"), "pas du vdf {{{");
        let old = root.join("userdata/111/config/localconfig.vdf");
        let new = root.join("userdata/222/config/localconfig.vdf");
        write(&old, &LOCALCONFIG.replace("\"326\"", "\"5\""));
        write(&new, LOCALCONFIG);
        set_mtime(&old, 1_000_000);
        set_mtime(&new, 2_000_000);
        (tmp, root, lib2)
    }

    #[test]
    fn finds_apps_in_all_libraries() {
        let (_tmp, root, _) = fake_steam();
        let apps = installed_apps(&root);
        assert_eq!(apps.len(), 3);
        let itt = apps.iter().find(|a| a.app_id == 1426210).unwrap();
        assert_eq!(itt.name, "It Takes Two");
        assert_eq!(itt.install_dir, root.join("steamapps").join("common").join("ItTakesTwo"));
    }

    #[test]
    fn playtime_comes_from_most_recent_localconfig() {
        let (_tmp, root, _) = fake_steam();
        assert_eq!(playtimes(&root)[&1544020], (326 * 60, Some(1730000000)));
    }

    #[test]
    fn offline_exclusion() {
        let (_tmp, root, _) = fake_steam();
        let apps = installed_apps(&root);
        let by = |id| apps.iter().find(|a| a.app_id == id).unwrap();
        assert!(offline_excluded(by(228980)));
        assert!(!offline_excluded(by(1426210)));
        let proton = SteamApp { app_id: 1, name: "Proton 9.0".into(), manifest_path: PathBuf::new(), install_dir: PathBuf::new() };
        assert!(offline_excluded(&proton));
        let named = |id, n: &str| SteamApp { app_id: id, name: n.into(), manifest_path: PathBuf::new(), install_dir: PathBuf::new() };
        assert!(offline_excluded(&named(5, "Some VC Redistributable")));
        assert!(offline_excluded(&named(6, "Steam Linux Runtime 3.0 (sniper)")));
        assert!(offline_excluded(&named(7, "sTeAmWoRkS Shared")));
        assert!(!offline_excluded(&named(8, "Portal 2")));
    }

    #[test]
    fn playtimes_empty_without_userdata() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(playtimes(tmp.path()).is_empty());
    }

    fn single_library_vdf(path: &str) -> String {
        format!("\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{path}\"\n\t}}\n}}\n")
    }

    #[test]
    fn root_written_with_other_separators_is_not_duplicated() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Steam");
        write(&root.join("steamapps/appmanifest_1426210.acf"), MANIFEST);
        let alt = root.to_string_lossy().replace('\\', "/") + "/";
        write(&root.join("steamapps/libraryfolders.vdf"), &single_library_vdf(&alt));
        assert_eq!(library_dirs(&root).len(), 1);
        assert_eq!(installed_apps(&root).len(), 1);
    }

    #[cfg(windows)]
    #[test]
    fn root_written_in_other_case_is_not_duplicated() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Steam");
        write(&root.join("steamapps/appmanifest_1426210.acf"), MANIFEST);
        let upper = root.to_string_lossy().to_uppercase();
        write(&root.join("steamapps/libraryfolders.vdf"), &single_library_vdf(&vdf_path(Path::new(&upper))));
        // Registre en minuscules, VDF en majuscules : une seule entrée, graphie du VDF.
        let lower = PathBuf::from(root.to_string_lossy().to_lowercase());
        assert_eq!(library_dirs(&lower), vec![PathBuf::from(&upper)]);
        let ids: Vec<u32> = installed_apps(&lower).iter().map(|a| a.app_id).collect();
        assert_eq!(ids, vec![1426210]);
    }

    #[test]
    fn missing_libraryfolders_still_scans_root() {
        let (_tmp, root, _) = fake_steam();
        fs::remove_file(root.join("steamapps/libraryfolders.vdf")).unwrap();
        assert_eq!(library_dirs(&root), vec![root.clone()]);
        let ids: Vec<u32> = installed_apps(&root).iter().map(|a| a.app_id).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&1426210));
    }

    #[test]
    fn library_dirs_has_no_duplicates() {
        let (_tmp, root, lib2) = fake_steam();
        assert_eq!(library_dirs(&root), vec![root.clone(), lib2]);
    }

    // --- synchro dans la bibliothèque ---

    use crate::library::CURRENT_VERSION;

    fn empty_lib() -> Library {
        Library { version: CURRENT_VERSION, revision: 0, emulators: vec![], systems: vec![], games: vec![] }
    }

    fn app(id: u32, name: &str) -> SteamApp {
        SteamApp {
            app_id: id,
            name: name.into(),
            manifest_path: PathBuf::from(format!("C:/Steam/steamapps/appmanifest_{id}.acf")),
            install_dir: PathBuf::new(),
        }
    }

    fn three_apps() -> Vec<SteamApp> {
        vec![app(1426210, "It Takes Two"), app(1544020, "The Callisto Protocol"), app(228980, "Steamworks Common Redistributables")]
    }

    fn steam_game<'a>(lib: &'a Library, id: u32) -> &'a Game {
        lib.games.iter().find(|g| g.store.as_ref().is_some_and(|s| s.store == "steam" && s.id == id.to_string())).unwrap()
    }

    #[test]
    fn sync_adds_games_and_steam_system() {
        let mut lib = empty_lib();
        let types = HashMap::from([(1426210, StoreType::Game), (1544020, StoreType::Game), (228980, StoreType::Other)]);
        let report = sync_into(&mut lib, &three_apps(), &HashMap::new(), Some(&types));
        assert_eq!(report.added.len(), 2);
        assert_eq!(lib.games.len(), 2);
        let sys = lib.system("steam").unwrap();
        assert_eq!(sys.color, "#66C0F4");
        let g = steam_game(&lib, 1426210);
        assert_eq!(g.kind, LaunchKind::Store);
        assert_eq!(g.title, "It Takes Two");
        assert_eq!(g.system_id, "steam");
        assert_eq!(g.path, "C:/Steam/steamapps/appmanifest_1426210.acf");
        assert!(report.added.contains(&g.id));
    }

    #[test]
    fn sync_without_store_types_uses_offline_exclusion() {
        let mut lib = empty_lib();
        sync_into(&mut lib, &three_apps(), &HashMap::new(), None);
        assert_eq!(lib.games.len(), 2);
        assert!(!lib.games.iter().any(|g| g.store.as_ref().is_some_and(|s| s.id == "228980")));
    }

    #[test]
    fn resync_updates_playtime_without_duplicates() {
        let mut lib = empty_lib();
        sync_into(&mut lib, &three_apps(), &HashMap::from([(1426210, (100 * 60, Some(50)))]), None);
        let id = steam_game(&lib, 1426210).id.clone();
        lib.game_mut(&id).unwrap().favorite = true;
        lib.game_mut(&id).unwrap().last_played = Some(900);
        let report = sync_into(&mut lib, &three_apps(), &HashMap::from([(1426210, (400 * 60, Some(500)))]), None);
        assert_eq!(lib.games.len(), 2);
        assert!(report.added.is_empty());
        assert_eq!(report.updated, 1);
        let g = steam_game(&lib, 1426210);
        assert_eq!(g.playtime_secs, 400 * 60);
        assert_eq!(g.last_played, Some(900), "le plus grand des deux");
        assert!(g.favorite);
    }

    #[test]
    fn resync_with_identical_data_changes_nothing() {
        let mut lib = empty_lib();
        let pt = HashMap::from([(1426210, (100 * 60, Some(50)))]);
        sync_into(&mut lib, &three_apps(), &pt, None);
        let report = sync_into(&mut lib, &three_apps(), &pt, None);
        assert_eq!(report, SyncReport { added: vec![], updated: 0 });
    }

    #[test]
    fn uninstalled_app_keeps_game() {
        let mut lib = empty_lib();
        sync_into(&mut lib, &three_apps(), &HashMap::new(), None);
        let id = steam_game(&lib, 1426210).id.clone();
        lib.game_mut(&id).unwrap().favorite = true;
        sync_into(&mut lib, &[app(1544020, "The Callisto Protocol")], &HashMap::new(), None);
        assert_eq!(lib.games.len(), 2);
        assert!(steam_game(&lib, 1426210).favorite);
    }

    #[test]
    fn no_steam_creates_nothing() {
        let mut lib = empty_lib();
        let report = sync_into(&mut lib, &[], &HashMap::new(), None);
        assert!(lib.systems.is_empty() && lib.games.is_empty());
        assert_eq!(report, SyncReport { added: vec![], updated: 0 });
    }
}
