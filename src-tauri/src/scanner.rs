//! Recherche des jeux dans les dossiers d'un système.

use crate::library::{Library, System};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoundGame {
    pub path: String,
    pub title: String,
    pub system_id: String,
    pub size_bytes: u64,
}

const MAX_DEPTH: usize = 4;

/// Nettoie un nom de fichier « No-Intro » :
/// `God of War II (Europe, Australia) (En,Fr).iso` → `God of War II`.
pub fn clean_title(raw: &str) -> String {
    let mut out = String::new();
    let mut depth = 0i32;
    for c in raw.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = (depth - 1).max(0),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    let cleaned = out.replace('_', " ");
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let cleaned = cleaned.trim_matches(['-', '.', ' ']).to_string();
    if cleaned.is_empty() { raw.to_string() } else { cleaned }
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if depth < MAX_DEPTH {
                walk(&path, depth + 1, out);
            }
        } else {
            out.push(path);
        }
    }
}

fn is_eboot(path: &Path) -> bool {
    path.file_name()
        .map(|n| n.to_string_lossy().eq_ignore_ascii_case("eboot.bin"))
        .unwrap_or(false)
}

pub fn scan_system(lib: &Library, system: &System) -> Vec<FoundGame> {
    let known: std::collections::HashSet<String> =
        lib.games.iter().map(|g| g.path.to_lowercase()).collect();
    let exts: Vec<String> = system.extensions.iter().map(|e| e.to_lowercase()).collect();
    let mut found = Vec::new();

    for root in &system.rom_dirs {
        let root = Path::new(root);
        let mut files = Vec::new();
        walk(root, 0, &mut files);
        for file in files {
            let ext = file
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if !exts.contains(&ext) {
                continue;
            }
            // Un .bin n'est un jeu que s'il s'agit d'un EBOOT.BIN (PS3/PS4) ;
            // sinon c'est souvent une piste d'image disque.
            let eboot = is_eboot(&file);
            if ext == "bin" && !eboot {
                continue;
            }
            let path_str = file.to_string_lossy().into_owned();
            if known.contains(&path_str.to_lowercase()) {
                continue;
            }
            let title = if eboot {
                // Le titre est le premier dossier sous la racine.
                file.strip_prefix(root)
                    .ok()
                    .and_then(|rel| rel.components().next())
                    .map(|c| clean_title(&c.as_os_str().to_string_lossy()))
                    .unwrap_or_else(|| "Jeu sans nom".into())
            } else {
                clean_title(&file.file_stem().unwrap_or_default().to_string_lossy())
            };
            let size_bytes = fs::metadata(&file).map(|m| m.len()).unwrap_or(0);
            found.push(FoundGame { path: path_str, title, system_id: system.id.clone(), size_bytes });
        }
    }
    found.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    found
}

#[cfg(test)]
mod tests {
    use super::clean_title;

    #[test]
    fn strips_region_and_language_tags() {
        assert_eq!(clean_title("God of War II (Europe, Australia) (En,Fr,De,Es,It,Ru)"), "God of War II");
        assert_eq!(clean_title("[SuperPSX.com]-God of War 3"), "God of War 3");
        assert_eq!(clean_title("Devil May Cry 2 (Europe) (Disc 1)"), "Devil May Cry 2");
    }
}
