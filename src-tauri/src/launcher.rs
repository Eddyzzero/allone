//! Construit et lance la commande d'un jeu : émulateur + fichier du jeu,
//! ou exécutable natif pour un port PC.

use crate::library::{Game, LaunchKind, Library};
use std::path::Path;
use std::process::Command;

/// Commande prête à lancer, aussi renvoyée à l'interface pour l'afficher
/// (utile pour vérifier ce qui sera exécuté).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchPlan {
    pub program: String,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    /// Nom de l'émulateur, ou `None` pour un jeu natif.
    pub emulator: Option<String>,
}

impl LaunchPlan {
    pub fn to_command(&self) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args);
        if let Some(dir) = &self.working_dir {
            cmd.current_dir(dir);
        }
        cmd
    }
}

fn parent_dir(path: &str) -> Option<String> {
    Path::new(path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_string_lossy().into_owned())
}

pub fn plan(lib: &Library, game: &Game) -> Result<LaunchPlan, String> {
    match game.kind {
        LaunchKind::Store => {
            let store = game.store.as_ref().ok_or("Jeu de boutique sans identifiant")?;
            match store.store.as_str() {
                "steam" => Ok(LaunchPlan {
                    program: format!("steam://rungameid/{}", store.id),
                    args: vec![],
                    working_dir: None,
                    emulator: Some("Steam".into()),
                }),
                autre => Err(format!("Boutique « {autre} » non prise en charge")),
            }
        }
        LaunchKind::Native => Ok(LaunchPlan {
            program: game.path.clone(),
            args: game.args.clone(),
            // Un port a souvent besoin d'être lancé depuis son propre dossier
            // (fichiers de config, DLL, dossier _internal…).
            working_dir: game.working_dir.clone().or_else(|| parent_dir(&game.path)),
            emulator: None,
        }),
        LaunchKind::Emulated => {
            let emulator_id = game
                .emulator_override
                .clone()
                .or_else(|| lib.system(&game.system_id).and_then(|s| s.emulator_id.clone()))
                .ok_or_else(|| format!("Aucun émulateur n'est associé à « {} ».", game.system_id))?;
            let emu = lib
                .emulator(&emulator_id)
                .ok_or_else(|| format!("L'émulateur « {emulator_id} » n'est pas configuré."))?;
            let mut args: Vec<String> = emu.args.iter().map(|a| a.replace("{rom}", &game.path)).collect();
            if !emu.args.iter().any(|a| a.contains("{rom}")) {
                args.push(game.path.clone());
            }
            Ok(LaunchPlan {
                program: emu.exe_path.clone(),
                args,
                working_dir: emu.working_dir.clone().or_else(|| parent_dir(&emu.exe_path)),
                emulator: Some(emu.name.clone()),
            })
        }
    }
}

/// Vérifie que les fichiers nécessaires existent avant de lancer.
pub fn check(plan: &LaunchPlan, game: &Game) -> Result<(), String> {
    // Un jeu de boutique se lance par une URI : on vérifie seulement son manifeste.
    if game.kind == LaunchKind::Store {
        return if Path::new(&game.path).exists() { Ok(()) } else { Err("Jeu Steam désinstallé".into()) };
    }
    if !Path::new(&plan.program).exists() {
        return Err(match plan.emulator {
            Some(ref name) => format!("{name} est introuvable : {}", plan.program),
            None => format!("Exécutable introuvable : {}", plan.program),
        });
    }
    if game.kind == LaunchKind::Emulated && !Path::new(&game.path).exists() {
        return Err(format!("Fichier du jeu introuvable : {}", game.path));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::{Emulator, System};

    fn lib() -> Library {
        Library {
            version: crate::library::CURRENT_VERSION,
            revision: 0,
            emulators: vec![Emulator {
                id: "pcsx2".into(),
                name: "PCSX2".into(),
                exe_path: "C:/Emu/pcsx2-qt.exe".into(),
                args: vec!["-fullscreen".into(), "--".into(), "{rom}".into()],
                working_dir: None,
            }],
            systems: vec![System {
                id: "ps2".into(),
                name: "PlayStation 2".into(),
                short_name: "PS2".into(),
                color: "#fff".into(),
                year: None,
                emulator_id: Some("pcsx2".into()),
                rom_dirs: vec![],
                extensions: vec![],
                lb_platform: None,
                logo: None,
            }],
            games: vec![],
        }
    }

    fn game(kind: LaunchKind, path: &str) -> Game {
        serde_json::from_value(serde_json::json!({
            "id": "g", "title": "T", "systemId": "ps2", "kind": kind, "path": path
        }))
        .unwrap()
    }

    #[test]
    fn emulated_game_uses_system_emulator_template() {
        let p = plan(&lib(), &game(LaunchKind::Emulated, "D:/roms/gow.iso")).unwrap();
        assert_eq!(p.program, "C:/Emu/pcsx2-qt.exe");
        assert_eq!(p.args, vec!["-fullscreen", "--", "D:/roms/gow.iso"]);
        assert_eq!(p.working_dir.as_deref(), Some("C:/Emu"));
        assert_eq!(p.emulator.as_deref(), Some("PCSX2"));
    }

    #[test]
    fn native_game_runs_from_its_own_folder() {
        let p = plan(&lib(), &game(LaunchKind::Native, "D:/ports/gow3/Play.exe")).unwrap();
        assert_eq!(p.program, "D:/ports/gow3/Play.exe");
        assert!(p.args.is_empty());
        assert_eq!(p.working_dir.as_deref(), Some("D:/ports/gow3"));
        assert!(p.emulator.is_none());
    }

    #[test]
    fn missing_emulator_is_an_error() {
        let mut l = lib();
        l.emulators.clear();
        assert!(plan(&l, &game(LaunchKind::Emulated, "x.iso")).is_err());
    }

    #[test]
    fn store_game_launches_through_steam_uri() {
        let mut g = game(LaunchKind::Store, "C:/Steam/steamapps/appmanifest_1426210.acf");
        g.store = Some(crate::library::StoreRef { store: "steam".into(), id: "1426210".into() });
        let p = plan(&lib(), &g).unwrap();
        assert_eq!(p.program, "steam://rungameid/1426210");
        assert!(p.args.is_empty() && p.working_dir.is_none());
        assert_eq!(p.emulator.as_deref(), Some("Steam"));
        assert_eq!(check(&p, &g), Err("Jeu Steam désinstallé".to_string()));
    }
}
