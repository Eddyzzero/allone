//! Import du temps de jeu enregistré par Playnite (`library/games.db`, base LiteDB).
//!
//! Le temps importé est mémorisé par jeu (`playnite_secs`) : relancer l'import
//! remplace l'ancien apport de Playnite au lieu de l'ajouter une seconde fois, et le
//! temps compté par allone lui-même est conservé.

use crate::launchbox::matching::normalize;
use crate::library::{LaunchKind, Library};
use crate::litedb::{self, Bson, Doc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Extension Steam intégrée à Playnite.
const STEAM_PLUGIN: &str = "cb91dfc9-b977-43bf-8e70-55f46e410fab";

/// Un jeu de la bibliothèque Playnite, réduit à ce qui sert à l'import.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayniteGame {
    pub name: String,
    pub playtime_secs: u64,
    /// Dernière partie (secondes Unix).
    pub last_played: Option<i64>,
    /// Appid quand le jeu vient de l'extension Steam.
    pub steam_app_id: Option<String>,
    /// Fichiers du jeu (ROM, exécutable), `{InstallDir}` remplacé.
    pub paths: Vec<String>,
}

#[derive(Debug, Default, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    /// Jeux allone dont le temps de jeu a changé.
    pub updated: usize,
    /// Jeux Playnite retrouvés dans allone (changés ou non).
    pub matched: usize,
    /// Temps ajouté au total, en secondes (négatif si Playnite en compte moins qu'avant).
    pub added_secs: i64,
    /// Jeux Steam laissés de côté : leur temps vient déjà de Steam.
    pub steam_skipped: usize,
    /// Jeux Playnite avec du temps de jeu mais absents d'allone.
    pub unmatched: Vec<String>,
}

/// Dossier `library` de Playnite (installation standard), s'il existe.
pub fn library_dir() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA")?;
    let dir = PathBuf::from(appdata).join("Playnite").join("library");
    dir.join("games.db").is_file().then_some(dir)
}

/// Lit les jeux de `games.db`. Playnite verrouille ce fichier tant qu'il est ouvert.
pub fn read_games(library_dir: &Path) -> Result<Vec<PlayniteGame>, String> {
    let path = library_dir.join("games.db");
    let data = std::fs::read(&path).map_err(|e| {
        if e.raw_os_error() == Some(32) {
            "Playnite est ouvert : ferme-le complètement (aussi dans la zone de notification), puis réessaie.".to_string()
        } else {
            format!("Lecture de {} impossible : {e}", path.display())
        }
    })?;
    let docs = litedb::documents(&data).map_err(|e| format!("Base Playnite illisible : {e}"))?;
    Ok(docs.iter().filter_map(game_from_doc).collect())
}

/// Identifiant .NET (`Guid`) stocké en binaire, sous sa forme texte habituelle.
fn guid(bytes: &[u8]) -> Option<String> {
    let b: &[u8; 16] = bytes.try_into().ok()?;
    let hex = |s: &[u8]| s.iter().map(|x| format!("{x:02x}")).collect::<String>();
    let rev = |s: &[u8]| s.iter().rev().copied().collect::<Vec<u8>>();
    Some(format!("{}-{}-{}-{}-{}", hex(&rev(&b[0..4])), hex(&rev(&b[4..6])), hex(&rev(&b[6..8])), hex(&b[8..10]), hex(&b[10..16])))
}

fn game_from_doc(doc: &Doc) -> Option<PlayniteGame> {
    let name = doc.get("Name")?.as_str()?.trim().to_string();
    if name.is_empty() {
        return None;
    }
    let playtime_secs = doc.get("Playtime").and_then(Bson::as_i64).unwrap_or(0).max(0) as u64;
    let last_played = match doc.get("LastActivity") {
        Some(Bson::DateTime(ms)) => Some(ms.div_euclid(1000)),
        _ => None,
    };
    let plugin = match doc.get("PluginId") {
        Some(Bson::Binary(b)) => guid(b),
        _ => None,
    };
    let game_id = doc.get("GameId").and_then(Bson::as_str).map(str::to_string);
    let steam_app_id = if plugin.as_deref() == Some(STEAM_PLUGIN) { game_id } else { None };

    let install_dir = doc.get("InstallDirectory").and_then(Bson::as_str).unwrap_or("");
    let expand = |p: &str| p.replace("{InstallDir}", install_dir);
    let mut paths = Vec::new();
    for list in ["Roms", "GameActions"] {
        for item in doc.get(list).map(Bson::as_array).unwrap_or(&[]) {
            if let Some(p) = item.as_doc().and_then(|d| d.get("Path")).and_then(Bson::as_str) {
                if !p.trim().is_empty() && !p.contains("://") {
                    paths.push(expand(p));
                }
            }
        }
    }
    Some(PlayniteGame { name, playtime_secs, last_played, steam_app_id, paths })
}

/// Forme comparable d'un chemin Windows : minuscules, `\` uniquement, sans séparateur doublé.
fn norm_path(p: &str) -> String {
    let mut out = String::with_capacity(p.len());
    for c in p.trim().chars().flat_map(char::to_lowercase) {
        let c = if c == '/' { '\\' } else { c };
        if c == '\\' && out.ends_with('\\') {
            continue;
        }
        out.push(c);
    }
    out.trim_end_matches('\\').to_string()
}

/// Reporte le temps de jeu Playnite sur les jeux allone correspondants.
///
/// Correspondance : même fichier de jeu, sinon même titre (seulement s'il est unique des
/// deux côtés). Les jeux Steam sont ignorés : la synchro Steam fait déjà foi.
pub fn import_into(lib: &mut Library, games: &[PlayniteGame]) -> ImportReport {
    let mut report = ImportReport::default();
    // Jeux allone candidats (hors boutique), par chemin et par titre.
    let mut by_path: HashMap<String, usize> = HashMap::new();
    let mut by_title: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, g) in lib.games.iter().enumerate() {
        if g.kind == LaunchKind::Store {
            continue;
        }
        by_path.insert(norm_path(&g.path), i);
        by_title.entry(normalize(&g.title)).or_default().push(i);
    }
    let mut pn_titles: HashMap<String, usize> = HashMap::new();
    for pg in games.iter().filter(|g| g.steam_app_id.is_none()) {
        *pn_titles.entry(normalize(&pg.name)).or_default() += 1;
    }

    let mut taken = vec![false; lib.games.len()];
    // Deux passes : d'abord les chemins (sûrs), ensuite les titres pour ce qui reste.
    let mut pending: Vec<&PlayniteGame> = Vec::new();
    let mut found: Vec<(usize, &PlayniteGame)> = Vec::new();
    for pg in games {
        if pg.steam_app_id.is_some() {
            if pg.playtime_secs > 0 {
                report.steam_skipped += 1;
            }
            continue;
        }
        match pg.paths.iter().find_map(|p| by_path.get(&norm_path(p)).copied().filter(|&i| !taken[i])) {
            Some(i) => {
                taken[i] = true;
                found.push((i, pg));
            }
            None => pending.push(pg),
        }
    }
    for pg in pending {
        let key = normalize(&pg.name);
        let candidates: Vec<usize> = by_title.get(&key).map(|v| v.iter().copied().filter(|&i| !taken[i]).collect()).unwrap_or_default();
        if let ([i], Some(1)) = (candidates.as_slice(), pn_titles.get(&key)) {
            taken[*i] = true;
            found.push((*i, pg));
        } else if pg.playtime_secs > 0 {
            report.unmatched.push(pg.name.clone());
        }
    }

    for (i, pg) in found {
        report.matched += 1;
        let g = &mut lib.games[i];
        let before = (g.playtime_secs, g.playnite_secs, g.last_played);
        let own = g.playtime_secs.saturating_sub(g.playnite_secs);
        g.playtime_secs = own + pg.playtime_secs;
        g.playnite_secs = pg.playtime_secs;
        report.added_secs += g.playtime_secs as i64 - before.0 as i64;
        if pg.playtime_secs > 0 {
            g.last_played = g.last_played.max(pg.last_played);
        }
        if before != (g.playtime_secs, g.playnite_secs, g.last_played) {
            report.updated += 1;
        }
    }
    report.unmatched.sort();
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::{CURRENT_VERSION, Game};
    use crate::litedb::tests::{arr, date, doc, i64v, litedb_file, s};
    use serde_json::json;

    fn game(id: &str, title: &str, kind: &str, path: &str, secs: u64) -> Game {
        serde_json::from_value(json!({
            "id": id, "title": title, "systemId": "ps2", "kind": kind, "path": path, "playtimeSecs": secs
        }))
        .unwrap()
    }

    fn lib(games: Vec<Game>) -> Library {
        Library { version: CURRENT_VERSION, revision: 0, emulators: vec![], systems: vec![], games }
    }

    fn pn(name: &str, secs: u64, paths: &[&str]) -> PlayniteGame {
        PlayniteGame {
            name: name.into(),
            playtime_secs: secs,
            last_played: Some(1_790_000_000),
            steam_app_id: None,
            paths: paths.iter().map(|p| p.to_string()).collect(),
        }
    }

    #[test]
    fn steam_plugin_guid_is_decoded() {
        let bytes = [0xc9, 0xdf, 0x91, 0xcb, 0x77, 0xb9, 0xbf, 0x43, 0x8e, 0x70, 0x55, 0xf4, 0x6e, 0x41, 0x0f, 0xab];
        assert_eq!(guid(&bytes).as_deref(), Some(STEAM_PLUGIN));
    }

    #[test]
    fn reads_games_from_a_litedb_file() {
        let steam_guid = vec![0xc9, 0xdf, 0x91, 0xcb, 0x77, 0xb9, 0xbf, 0x43, 0x8e, 0x70, 0x55, 0xf4, 0x6e, 0x41, 0x0f, 0xab];
        let mut bin = (16i32).to_le_bytes().to_vec();
        bin.push(3);
        bin.extend(steam_guid);
        let (short, _) = doc(vec![
            ("Name", s("Callisto")),
            ("Playtime", i64v(600)),
            ("GameId", s("1544020")),
            ("PluginId", (bin, 0x05)),
        ]);
        let big = "d".repeat(6000);
        let (long, _) = doc(vec![
            ("Name", s("God of War")),
            ("Description", s(&big)),
            ("Playtime", i64v(59935)),
            ("LastActivity", date(1_792_575_752_171)),
            ("InstallDirectory", s("C:\\Jeux\\PS2\\")),
            ("Roms", arr(vec![doc(vec![("Name", s("GoW")), ("Path", s("{InstallDir}\\God of War.iso"))])])),
        ]);
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("games.db"), litedb_file(&short, &long)).unwrap();
        let games = read_games(dir.path()).unwrap();
        assert_eq!(games[0].steam_app_id.as_deref(), Some("1544020"));
        assert_eq!(games[1].name, "God of War");
        assert_eq!(games[1].playtime_secs, 59935);
        assert_eq!(games[1].last_played, Some(1_792_575_752));
        assert_eq!(games[1].paths, vec!["C:\\Jeux\\PS2\\\\God of War.iso".to_string()]);
    }

    #[test]
    fn matches_by_path_then_unique_title() {
        let mut l = lib(vec![
            game("a", "God of War", "emulated", "C:/Jeux/PS2/God of War.iso", 0),
            game("b", "God of War: Ghost of Sparta", "emulated", "D:\\psp\\gos.chd", 0),
            game("c", "God of War III Remastered", "native", "C:\\ps4\\port\\Play.exe", 38),
        ]);
        let report = import_into(&mut l, &[
            pn("God of War (PAL)", 59935, &["c:\\jeux\\ps2\\\\GOD OF WAR.iso"]),
            pn("God of War - Ghost of Sparta", 27209, &["E:\\autre\\gos.chd"]),
            pn("God of War® III Remastered", 2, &["C:\\ps4\\CUSA01623\\eboot.bin"]),
            pn("Freekstyle", 4082, &[]),
        ]);
        assert_eq!(l.games[0].playtime_secs, 59935);
        assert_eq!(l.games[1].playtime_secs, 27209);
        assert_eq!(l.games[2].playtime_secs, 40, "le temps compté par allone est gardé");
        assert_eq!(l.games[0].last_played, Some(1_790_000_000));
        assert_eq!((report.matched, report.updated), (3, 3));
        assert_eq!(report.added_secs, 59935 + 27209 + 2);
        assert_eq!(report.unmatched, vec!["Freekstyle".to_string()]);
    }

    #[test]
    fn reimport_does_not_double_count() {
        let mut l = lib(vec![game("a", "Def Jam", "emulated", "x.iso", 100)]);
        import_into(&mut l, &[pn("Def Jam", 22422, &["x.iso"])]);
        assert_eq!(l.games[0].playtime_secs, 22522);
        // On joue 50 s depuis allone, puis Playnite a compté 78 s de plus de son côté.
        l.games[0].playtime_secs += 50;
        let r = import_into(&mut l, &[pn("Def Jam", 22500, &["x.iso"])]);
        assert_eq!(l.games[0].playtime_secs, 100 + 50 + 22500);
        assert_eq!(r.added_secs, 78);
        let r = import_into(&mut l, &[pn("Def Jam", 22500, &["x.iso"])]);
        assert_eq!((r.matched, r.updated, r.added_secs), (1, 0, 0));
    }

    #[test]
    fn steam_and_ambiguous_titles_are_left_alone() {
        let mut l = lib(vec![
            game("s", "The Callisto Protocol", "store", "m.acf", 19560),
            game("a", "Devil May Cry", "emulated", "a.iso", 0),
            game("b", "Devil May Cry", "emulated", "b.iso", 0),
        ]);
        let mut steam = pn("The Callisto Protocol", 19534, &[]);
        steam.steam_app_id = Some("1544020".into());
        let r = import_into(&mut l, &[steam, pn("Devil May Cry", 2166, &[])]);
        assert_eq!(l.games[0].playtime_secs, 19560);
        assert!(l.games[1..].iter().all(|g| g.playtime_secs == 0));
        assert_eq!(r.steam_skipped, 1);
        assert_eq!(r.unmatched, vec!["Devil May Cry".to_string()]);
    }

    #[test]
    fn locked_or_missing_file_gives_a_message() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_games(dir.path()).unwrap_err().contains("games.db"));
    }
}
