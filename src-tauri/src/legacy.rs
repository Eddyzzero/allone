//! Reprise des données de l'ancien identifiant (`com.jhon.allone`) vers le nouveau
//! (`com.allone.launcher`) : copie du dossier de données et réécriture des chemins absolus.

use std::fs;
use std::path::{Path, PathBuf};

/// Compare deux caractères en ignorant `/` vs `\` et, sous Windows, la casse.
fn same_char(a: char, b: char) -> bool {
    let sep = |c: char| c == '/' || c == '\\';
    if sep(a) || sep(b) {
        return sep(a) && sep(b);
    }
    if cfg!(windows) {
        a.to_lowercase().eq(b.to_lowercase())
    } else {
        a == b
    }
}

/// Si `path` commence par le dossier `old` (à une frontière de séparateur), renvoie le même
/// chemin placé sous `new`. Les autres chemins (ex. `/systems/ps2.svg`) renvoient `None`.
pub fn rewrite_prefix(path: &str, old: &Path, new: &Path) -> Option<String> {
    let old_s = old.to_string_lossy();
    let old_s = old_s.trim_end_matches(['/', '\\']);
    if old_s.is_empty() {
        return None;
    }
    let mut p = path.chars();
    for oc in old_s.chars() {
        if !same_char(p.next()?, oc) {
            return None;
        }
    }
    let rest: String = p.collect();
    if !(rest.is_empty() || rest.starts_with(['/', '\\'])) {
        return None;
    }
    let new_s = new.to_string_lossy();
    Some(format!("{}{}", new_s.trim_end_matches(['/', '\\']), rest))
}

/// Copie récursive sans suivre les liens symboliques ; ignore `*.part` / `*.tmp`.
/// `skip_root` : nom de fichier à ne pas copier à la racine (library.json, écrit en dernier).
fn copy_dir(src: &Path, dst: &Path, skip_root: Option<&str>) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("création de {} : {e}", dst.display()))?;
    for entry in fs::read_dir(src).map_err(|e| format!("lecture de {} : {e}", src.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let name_s = name.to_string_lossy();
        if skip_root == Some(name_s.as_ref()) {
            continue;
        }
        let ft = entry.file_type().map_err(|e| e.to_string())?;
        if ft.is_symlink() {
            continue;
        }
        let target = dst.join(&name);
        if ft.is_dir() {
            copy_dir(&entry.path(), &target, None)?;
        } else if ft.is_file() {
            let lower = name_s.to_lowercase();
            if lower.ends_with(".part") || lower.ends_with(".tmp") {
                continue;
            }
            fs::copy(entry.path(), &target)
                .map_err(|e| format!("copie de {} : {e}", entry.path().display()))?;
        }
    }
    Ok(())
}

fn rewrite_field(obj: &mut serde_json::Value, key: &str, old: &Path, new: &Path) {
    if let Some(serde_json::Value::String(s)) = obj.get_mut(key) {
        if let Some(n) = rewrite_prefix(s, old, new) {
            *s = n;
        }
    }
}

/// Reprend les données de `old_dir` dans `new_dir` au premier lancement après le changement
/// d'identifiant. `Ok(true)` si une copie a eu lieu. L'ancien dossier n'est jamais modifié.
/// `library.json` est écrit en dernier (via un fichier temporaire) : il sert de marqueur « migration
/// terminée ». En cas d'`Err`, l'appelant ne doit RIEN écrire dans `new_dir` (voir
/// `choose_data_dir`) : la migration est alors retentée au lancement suivant.
pub fn migrate_data_dir(new_dir: &Path, old_dir: &Path) -> Result<bool, String> {
    let new_lib = new_dir.join("library.json");
    let old_lib = old_dir.join("library.json");
    if new_lib.exists() || !old_lib.is_file() {
        return Ok(false);
    }
    if new_dir.starts_with(old_dir) || old_dir.starts_with(new_dir) {
        return Ok(false);
    }
    copy_dir(old_dir, new_dir, Some("library.json"))?;

    let text = fs::read_to_string(&old_lib).map_err(|e| format!("lecture de library.json : {e}"))?;
    let mut json: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("library.json invalide : {e}"))?;
    for (list, keys) in [("games", &["cover", "hero", "logo"][..]), ("systems", &["logo"][..])] {
        if let Some(items) = json.get_mut(list).and_then(|v| v.as_array_mut()) {
            for item in items {
                for key in keys {
                    rewrite_field(item, key, old_dir, new_dir);
                }
            }
        }
    }
    let out = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
    let tmp = new_dir.join("library.json.tmp");
    fs::write(&tmp, out).map_err(|e| format!("écriture de library.json : {e}"))?;
    fs::rename(&tmp, &new_lib).map_err(|e| format!("finalisation de library.json : {e}"))?;
    Ok(true)
}

/// Choisit le dossier de données de la session. Si la migration a échoué alors que l'ancien
/// dossier contient une bibliothèque, on travaille dans l'ancien dossier (sans rien écrire dans le
/// nouveau) pour que la migration soit retentée au prochain lancement.
pub fn choose_data_dir(new_dir: &Path, old_dir: &Path, migration: &Result<bool, String>) -> PathBuf {
    if migration.is_err() && old_dir.join("library.json").is_file() {
        old_dir.to_path_buf()
    } else {
        new_dir.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(dir: &Path, rel: &str) -> String {
        dir.join(rel).to_string_lossy().into_owned()
    }

    #[test]
    fn copies_and_rewrites_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let old = tmp.path().join("com.jhon.allone");
        let new = tmp.path().join("com.allone.launcher");
        fs::create_dir_all(old.join("media/g1")).unwrap();
        fs::create_dir_all(old.join("logos")).unwrap();
        fs::write(old.join("media/g1/cover.jpg"), b"cover").unwrap();
        fs::write(old.join("media/g1/cover.jpg.part"), b"x").unwrap();
        fs::write(old.join("logos/gc.svg"), b"<svg/>").unwrap();
        let lib = serde_json::json!({
            "version": 3,
            "extra": "garde-moi",
            "systems": [{"id": "gc", "logo": p(&old, "logos/gc.svg")},
                        {"id": "ps2", "logo": "/systems/ps2.svg"}],
            "games": [{"id": "g1", "cover": p(&old, "media/g1/cover.jpg"), "hero": null,
                       "logo": "C:\\ailleurs\\logo.png"}]
        });
        fs::write(old.join("library.json"), lib.to_string()).unwrap();

        assert_eq!(migrate_data_dir(&new, &old), Ok(true));

        assert_eq!(fs::read(new.join("media/g1/cover.jpg")).unwrap(), b"cover");
        assert!(new.join("logos/gc.svg").is_file());
        assert!(!new.join("media/g1/cover.jpg.part").exists());
        let got: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(new.join("library.json")).unwrap()).unwrap();
        assert_eq!(got["extra"], "garde-moi");
        assert_eq!(got["systems"][0]["logo"], p(&new, "logos/gc.svg"));
        assert_eq!(got["systems"][1]["logo"], "/systems/ps2.svg");
        assert_eq!(got["games"][0]["cover"], p(&new, "media/g1/cover.jpg"));
        assert!(got["games"][0]["hero"].is_null());
        assert_eq!(got["games"][0]["logo"], "C:\\ailleurs\\logo.png");
        // ancien dossier intact
        let old_got: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(old.join("library.json")).unwrap()).unwrap();
        assert_eq!(old_got, lib);
        assert!(old.join("media/g1/cover.jpg").is_file());
        assert!(!new.join("library.json.tmp").exists());
    }

    #[test]
    fn skips_when_new_library_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let old = tmp.path().join("old");
        let new = tmp.path().join("new");
        fs::create_dir_all(&old).unwrap();
        fs::create_dir_all(&new).unwrap();
        fs::write(old.join("library.json"), "{}").unwrap();
        fs::write(old.join("autre.txt"), "a").unwrap();
        fs::write(new.join("library.json"), "{\"nouveau\":1}").unwrap();
        assert_eq!(migrate_data_dir(&new, &old), Ok(false));
        assert_eq!(fs::read_to_string(new.join("library.json")).unwrap(), "{\"nouveau\":1}");
        assert!(!new.join("autre.txt").exists());
    }

    #[test]
    fn skips_without_old_library() {
        let tmp = tempfile::tempdir().unwrap();
        let old = tmp.path().join("old");
        let new = tmp.path().join("new");
        fs::create_dir_all(&old).unwrap();
        assert_eq!(migrate_data_dir(&new, &old), Ok(false));
        assert!(!new.exists());
        let missing = tmp.path().join("absent");
        assert_eq!(migrate_data_dir(&new, &missing), Ok(false));
    }

    #[test]
    fn rewrite_prefix_ignores_other_paths() {
        let old = PathBuf::from(if cfg!(windows) { "C:\\Data\\old" } else { "/data/old" });
        let new = PathBuf::from(if cfg!(windows) { "C:\\Data\\new" } else { "/data/new" });
        assert_eq!(rewrite_prefix("/systems/ps2.svg", &old, &new), None);
        assert_eq!(rewrite_prefix(&p(Path::new("/zzz"), "media/a.jpg"), &old, &new), None);
        // frontière de dossier : « old2 » n'est pas « old »
        let sibling = format!("{}2/x.jpg", old.to_string_lossy());
        assert_eq!(rewrite_prefix(&sibling, &old, &new), None);
    }

    #[cfg(windows)]
    #[test]
    fn rewrite_prefix_windows_tolerant() {
        let old = PathBuf::from("C:\\Users\\X\\AppData\\Roaming\\com.jhon.allone");
        let new = PathBuf::from("C:\\Users\\X\\AppData\\Roaming\\com.allone.launcher");
        let got = rewrite_prefix(
            "c:/users/x/appdata/roaming/COM.JHON.ALLONE/media/s/cover.jpg",
            &old,
            &new,
        );
        assert_eq!(
            got.as_deref(),
            Some("C:\\Users\\X\\AppData\\Roaming\\com.allone.launcher/media/s/cover.jpg")
        );
    }

    #[test]
    fn choose_data_dir_cases() {
        let tmp = tempfile::tempdir().unwrap();
        let old = tmp.path().join("old");
        let new = tmp.path().join("new");
        let err: Result<bool, String> = Err("boum".into());
        // Err sans ancienne bibliothèque -> nouveau
        assert_eq!(choose_data_dir(&new, &old, &err), new);
        fs::create_dir_all(&old).unwrap();
        fs::write(old.join("library.json"), "{}").unwrap();
        // Err avec ancienne bibliothèque -> ancien
        assert_eq!(choose_data_dir(&new, &old, &err), old);
        // Ok -> nouveau
        assert_eq!(choose_data_dir(&new, &old, &Ok(true)), new);
        assert_eq!(choose_data_dir(&new, &old, &Ok(false)), new);
    }
}
