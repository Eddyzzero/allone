//! Modèle de la bibliothèque et persistance dans un fichier JSON
//! (`library.json` dans le dossier de données de l'application).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Un émulateur installé sur la machine.
///
/// `args` est un gabarit : chaque élément est un argument, et `{rom}` est
/// remplacé par le chemin du jeu au lancement.
/// Exemple PCSX2 : `["-nogui", "-fullscreen", "--", "{rom}"]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Emulator {
    pub id: String,
    pub name: String,
    pub exe_path: String,
    pub args: Vec<String>,
    /// Dossier de travail ; par défaut, le dossier de l'exécutable.
    #[serde(default)]
    pub working_dir: Option<String>,
}

/// Une console (ou une famille de jeux) et l'émulateur qui la lance par défaut.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct System {
    pub id: String,
    pub name: String,
    pub short_name: String,
    pub color: String,
    #[serde(default)]
    pub year: Option<u16>,
    #[serde(default)]
    pub emulator_id: Option<String>,
    /// Dossiers où chercher des jeux pour ce système.
    #[serde(default)]
    pub rom_dirs: Vec<String>,
    /// Extensions reconnues (sans le point), ex. `["iso", "chd"]`.
    #[serde(default)]
    pub extensions: Vec<String>,
    /// Nom de la plateforme dans la base LaunchBox (ex. « Sony Playstation 2 »).
    #[serde(default)]
    pub lb_platform: Option<String>,
    /// Logo du système : chemin d'une image livrée (`/systems/ps2.svg`) ou fichier local.
    #[serde(default)]
    pub logo: Option<String>,
}

/// Comment un jeu se lance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LaunchKind {
    /// Un fichier de jeu ouvert par l'émulateur du système.
    Emulated,
    /// Un exécutable natif (port PC, jeu Windows/Linux) lancé directement.
    Native,
    /// Un jeu d'une boutique (Steam…), lancé par la boutique elle-même.
    Store,
}

/// Version courante du format de `library.json`.
pub const CURRENT_VERSION: u32 = 3;

/// Référence d'un jeu dans une boutique (ex. Steam, appid).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreRef {
    pub store: String,
    pub id: String,
}

/// Source des informations et images d'un jeu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MetaSource {
    Launchbox,
    Steam,
}

/// Correspondance retenue avec une base d'infos ; `locked` la protège
/// contre toute récupération ultérieure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaRef {
    pub source: MetaSource,
    pub id: String,
    #[serde(default)]
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: String,
    pub title: String,
    pub system_id: String,
    pub kind: LaunchKind,
    /// Fichier du jeu (ROM, ISO, EBOOT.BIN…) ou exécutable du port.
    pub path: String,
    /// Arguments supplémentaires (jeu natif) ; ignorés pour un jeu émulé.
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub working_dir: Option<String>,
    /// Pour forcer un autre émulateur que celui du système.
    #[serde(default)]
    pub emulator_override: Option<String>,
    /// Jaquette (format portrait) et image large pour le carrousel.
    #[serde(default)]
    pub cover: Option<String>,
    #[serde(default)]
    pub hero: Option<String>,
    /// Logo du jeu (image transparente).
    #[serde(default)]
    pub logo: Option<String>,
    /// Pour un jeu de boutique : la boutique et l'identifiant du jeu.
    #[serde(default)]
    pub store: Option<StoreRef>,
    /// Correspondance avec une base d'infos (LaunchBox, Steam).
    #[serde(default)]
    pub metadata: Option<MetaRef>,
    #[serde(default)]
    pub year: Option<u16>,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub playtime_secs: u64,
    /// Part de `playtime_secs` importée de Playnite (pour ne pas la compter deux fois).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub playnite_secs: u64,
    /// Horodatage Unix (secondes).
    #[serde(default)]
    pub last_played: Option<i64>,
    #[serde(default)]
    pub added_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub version: u32,
    /// Compteur incrémenté à chaque modification ; sert à refuser un
    /// enregistrement de l'interface fait sur une version périmée.
    #[serde(default)]
    pub revision: u64,
    pub emulators: Vec<Emulator>,
    pub systems: Vec<System>,
    pub games: Vec<Game>,
}

impl Library {
    pub fn load_or_seed(path: &Path) -> Library {
        match fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str(&text) {
                Ok(mut lib) => {
                    if Library::migrate(&mut lib) {
                        let _ = lib.save(path);
                    }
                    lib
                }
                Err(err) => {
                    // On garde le fichier illisible de côté plutôt que de l'écraser.
                    let backup = path.with_extension("json.bak");
                    let _ = fs::rename(path, &backup);
                    eprintln!("library.json illisible ({err}), sauvegardé dans {backup:?}");
                    seed()
                }
            },
            Err(_) => seed(),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        // Écriture atomique : fichier temporaire puis renommage.
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, text).map_err(|e| e.to_string())?;
        fs::rename(&tmp, path).map_err(|e| e.to_string())
    }

    /// Met la bibliothèque au format courant ; vrai si elle a changé.
    pub fn migrate(&mut self) -> bool {
        if self.version >= CURRENT_VERSION {
            return false;
        }
        if self.version < 2 {
            // v1 : les images « /demo/… » de la bibliothèque de départ ne
            // sont plus livrées ; on les retire pour laisser place aux vraies.
            for g in &mut self.games {
                if g.cover.as_deref().is_some_and(|c| c.starts_with("/demo/")) {
                    g.cover = None;
                }
                if g.hero.as_deref().is_some_and(|c| c.starts_with("/demo/")) {
                    g.hero = None;
                }
            }
        }
        if self.version < 3 {
            // v3 : la plateforme LaunchBox et le logo deviennent des données du système.
            for s in &mut self.systems {
                let platform = match s.id.as_str() {
                    "ps2" => Some("Sony Playstation 2"),
                    "ps3" => Some("Sony Playstation 3"),
                    "ps4" => Some("Sony Playstation 4"),
                    "psp" => Some("Sony PSP"),
                    _ => None,
                };
                if s.lb_platform.is_none() {
                    s.lb_platform = platform.map(String::from);
                }
                if s.logo.is_none() && matches!(s.id.as_str(), "ps2" | "ps3" | "ps4" | "psp" | "steam") {
                    s.logo = Some(format!("/systems/{}.svg", s.id));
                }
            }
        }
        self.version = CURRENT_VERSION;
        true
    }

    pub fn game(&self, id: &str) -> Option<&Game> {
        self.games.iter().find(|g| g.id == id)
    }

    pub fn game_mut(&mut self, id: &str) -> Option<&mut Game> {
        self.games.iter_mut().find(|g| g.id == id)
    }

    pub fn system(&self, id: &str) -> Option<&System> {
        self.systems.iter().find(|s| s.id == id)
    }

    pub fn emulator(&self, id: &str) -> Option<&Emulator> {
        self.emulators.iter().find(|e| e.id == id)
    }
}

/// Accepte un enregistrement venu de l'interface seulement s'il part de la
/// révision courante ; renvoie alors la bibliothèque avec `revision + 1`.
pub fn accept_save(current: &Library, mut incoming: Library) -> Result<Library, String> {
    if incoming.revision != current.revision {
        return Err("STALE".into());
    }
    incoming.revision = current.revision + 1;
    Ok(incoming)
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn new_id(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{prefix}-{nanos:x}")
}

/// Bibliothèque de départ, générique : les consoles PlayStation usuelles et leurs
/// émulateurs, sans chemin ni jeu. L'utilisateur renseigne ses propres dossiers
/// et exécutables depuis les réglages.
fn seed() -> Library {
    let emu = |id: &str, name: &str, args: &[&str]| Emulator {
        id: id.into(),
        name: name.into(),
        exe_path: String::new(),
        args: args.iter().map(|a| a.to_string()).collect(),
        working_dir: None,
    };
    let emulators = vec![
        emu("pcsx2", "PCSX2", &["-nogui", "-fullscreen", "--", "{rom}"]),
        emu("rpcs3", "RPCS3", &["--no-gui", "{rom}"]),
        emu("shadps4", "shadPS4", &["-g", "{rom}"]),
        emu("ppsspp", "PPSSPP", &["--fullscreen", "{rom}"]),
    ];

    let sys = |id: &str, name: &str, short: &str, color: &str, year: u16, emu: &str, ext: &[&str], platform: &str| System {
        id: id.into(),
        name: name.into(),
        short_name: short.into(),
        color: color.into(),
        year: Some(year),
        emulator_id: Some(emu.into()),
        rom_dirs: vec![],
        extensions: ext.iter().map(|e| e.to_string()).collect(),
        lb_platform: Some(platform.into()),
        logo: Some(format!("/systems/{id}.svg")),
    };
    let systems = vec![
        sys("ps2", "PlayStation 2", "PS2", "#8FD3FF", 2000, "pcsx2", &["iso", "chd", "bin", "cso"], "Sony Playstation 2"),
        sys("ps3", "PlayStation 3", "PS3", "#C9B6FF", 2006, "rpcs3", &["bin", "iso"], "Sony Playstation 3"),
        sys("ps4", "PlayStation 4", "PS4", "#7BE0A8", 2013, "shadps4", &["bin"], "Sony Playstation 4"),
        sys("psp", "PlayStation Portable", "PSP", "#F2B33D", 2004, "ppsspp", &["iso", "cso", "chd"], "Sony PSP"),
    ];

    Library { version: CURRENT_VERSION, revision: 0, emulators, systems, games: vec![] }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> Library {
        Library { version: CURRENT_VERSION, revision: 0, emulators: vec![], systems: vec![], games: vec![] }
    }

    const V1: &str = r#"{"version":1,"emulators":[],"systems":[],"games":[
        {"id":"g","title":"T","systemId":"ps2","kind":"emulated","path":"x.iso","cover":"/demo/gow1.jpg","hero":"/demo/gow1.jpg"},
        {"id":"h","title":"U","systemId":"ps2","kind":"emulated","path":"y.iso","cover":"C:/x/cover.jpg"}]}"#;

    #[test]
    fn v1_json_loads_with_defaults() {
        let lib: Library = serde_json::from_str(V1).unwrap();
        assert_eq!(lib.revision, 0);
        assert!(lib.games[0].logo.is_none() && lib.games[0].store.is_none() && lib.games[0].metadata.is_none());
    }

    #[test]
    fn migration_drops_demo_images_and_bumps_version() {
        let mut lib: Library = serde_json::from_str(V1).unwrap();
        assert!(lib.migrate());
        assert_eq!(lib.version, CURRENT_VERSION);
        assert!(lib.games[0].cover.is_none() && lib.games[0].hero.is_none());
        assert_eq!(lib.games[1].cover.as_deref(), Some("C:/x/cover.jpg"));
        assert!(!lib.migrate()); // idempotent
    }

    const V2: &str = r##"{"version":2,"revision":3,"emulators":[{"id":"e","name":"E","exePath":"e.exe","args":[]}],
        "systems":[
        {"id":"ps2","name":"PlayStation 2","shortName":"PS2","color":"#fff","emulatorId":"e"},
        {"id":"psp","name":"PSP","shortName":"PSP","color":"#fff"},
        {"id":"steam","name":"Steam","shortName":"Steam","color":"#fff"},
        {"id":"gc","name":"GameCube","shortName":"GC","color":"#fff"}],
        "games":[{"id":"g","title":"T","systemId":"ps2","kind":"emulated","path":"x.iso","cover":"C:/c.jpg"}]}"##;

    #[test]
    fn migration_v2_fills_platform_and_logo() {
        let mut lib: Library = serde_json::from_str(V2).unwrap();
        assert!(lib.migrate());
        assert_eq!(lib.version, 3);
        let s = |id: &str| lib.system(id).unwrap().clone();
        assert_eq!(s("ps2").lb_platform.as_deref(), Some("Sony Playstation 2"));
        assert_eq!(s("ps2").logo.as_deref(), Some("/systems/ps2.svg"));
        assert_eq!(s("psp").lb_platform.as_deref(), Some("Sony PSP"));
        assert_eq!(s("steam").lb_platform, None);
        assert_eq!(s("steam").logo.as_deref(), Some("/systems/steam.svg"));
        assert!(s("gc").lb_platform.is_none() && s("gc").logo.is_none());
        assert_eq!(lib.emulators.len(), 1);
        assert_eq!(lib.games.len(), 1);
        assert_eq!(lib.games[0].cover.as_deref(), Some("C:/c.jpg"));
        assert_eq!(lib.revision, 3);
        assert!(!lib.migrate()); // idempotent
    }

    #[test]
    fn migration_v1_chains_to_v3() {
        let mut lib: Library = serde_json::from_str(V1).unwrap();
        lib.systems.push(serde_json::from_str(r##"{"id":"ps2","name":"P","shortName":"P","color":"#fff"}"##).unwrap());
        assert!(lib.migrate());
        assert_eq!(lib.version, 3);
        assert!(lib.games[0].cover.is_none());
        assert_eq!(lib.systems[0].lb_platform.as_deref(), Some("Sony Playstation 2"));
    }

    #[test]
    fn migration_keeps_user_values() {
        let mut lib: Library = serde_json::from_str(V2).unwrap();
        lib.systems[0].lb_platform = Some("Perso".into());
        lib.systems[0].logo = Some("C:/mon.png".into());
        assert!(lib.migrate());
        assert_eq!(lib.systems[0].lb_platform.as_deref(), Some("Perso"));
        assert_eq!(lib.systems[0].logo.as_deref(), Some("C:/mon.png"));
    }

    #[test]
    fn seed_systems_have_platform_and_logo() {
        let lib = seed();
        assert!(lib.systems.iter().all(|s| s.lb_platform.is_some() && s.logo.is_some()));
    }

    #[test]
    fn stale_save_is_rejected() {
        let current = Library { revision: 5, ..empty() };
        assert_eq!(accept_save(&current, Library { revision: 4, ..empty() }).unwrap_err(), "STALE");
        assert_eq!(accept_save(&current, Library { revision: 5, ..empty() }).unwrap().revision, 6);
    }

    #[test]
    fn seed_is_generic() {
        let lib = seed();
        assert!(lib.games.is_empty());
        assert!(lib.emulators.iter().all(|e| e.exe_path.is_empty()));
        assert!(lib.systems.iter().all(|s| s.rom_dirs.is_empty()));
        let json = serde_json::to_string(&lib).unwrap();
        assert!(!json.contains("Documents") && !json.contains("Gaming"));
    }
}
