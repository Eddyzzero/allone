//! Index LaunchBox par plateforme, construit en flux depuis `Metadata.zip`.

use crate::media::ImageKind;
use quick_xml::Reader;
use quick_xml::events::Event;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DB_URL: &str = "https://gamesdb.launchbox-app.com/Metadata.zip";
const DB_NAME: &str = "Metadata.zip";

/// Un jeu de la base LaunchBox ; les images sont des noms de fichier LaunchBox.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LbGame {
    pub id: u64,
    pub name: String,
    pub alt_names: Vec<String>,
    pub platform: String,
    pub year: Option<u16>,
    pub overview: Option<String>,
    pub genres: Vec<String>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub cover: Option<String>,
    pub hero: Option<String>,
    pub logo: Option<String>,
}

/// Index des jeux des plateformes demandées. `built_from` = date de modification du zip source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LbIndex {
    pub built_from: i64,
    pub platforms: Vec<String>,
    pub games: Vec<LbGame>,
}

impl LbIndex {
    /// Lit un index ; `None` si le fichier est absent ou illisible.
    pub fn load(path: &Path) -> Option<LbIndex> {
        serde_json::from_slice(&fs::read(path).ok()?).ok()
    }

    /// Écrit l'index de façon atomique (fichier `.tmp` puis renommage).
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_vec(self).map_err(|e| format!("Sérialisation de l'index impossible : {e}"))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Création du dossier impossible : {e}"))?;
        }
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        let result = fs::write(&tmp, json).and_then(|_| fs::rename(&tmp, path));
        if let Err(e) = result {
            let _ = fs::remove_file(&tmp);
            return Err(format!("Écriture de l'index impossible : {e}"));
        }
        Ok(())
    }
}

/// Rang d'une région (plus petit = préféré) : Europe, France, World, North America, United States, autres, aucune.
fn region_rank(region: Option<&str>) -> u8 {
    match region {
        Some("Europe") => 0,
        Some("France") => 1,
        Some("World") => 2,
        Some("North America") => 3,
        Some("United States") => 4,
        Some(_) => 5,
        None => 6,
    }
}

fn kind_types(kind: ImageKind) -> &'static [&'static str] {
    match kind {
        ImageKind::Cover => &["Box - Front", "Box - Front - Reconstructed", "Fanart - Box - Front"],
        ImageKind::Hero => &["Fanart - Background", "Screenshot - Gameplay"],
        ImageKind::Logo => &["Clear Logo"],
    }
}

const KINDS: [ImageKind; 3] = [ImageKind::Cover, ImageKind::Hero, ImageKind::Logo];

/// Meilleur candidat courant d'un type d'image : (rang du type, rang de la région, fichier).
type Best = Option<(u8, u8, String)>;

/// Retient `file` s'il bat strictement le candidat courant (à égalité, le premier rencontré reste).
/// Règle unique de classement de la lecture en flux (vérifiée par `pick_image` dans les tests).
fn consider(best: &mut Best, kind: ImageKind, ty: &str, region: Option<&str>, file: &str) {
    let Some(type_rank) = kind_types(kind).iter().position(|t| *t == ty) else {
        return;
    };
    let key = (type_rank as u8, region_rank(region));
    if best.as_ref().is_none_or(|(t, r, _)| key < (*t, *r)) {
        *best = Some((key.0, key.1, file.to_string()));
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Record {
    Game,
    Alt,
    Image,
}

type Fields = Vec<(String, String)>;

fn field<'a>(fields: &'a Fields, key: &str) -> Option<&'a str> {
    fields.iter().find(|(k, _)| k == key).map(|(_, v)| v.trim()).filter(|v| !v.is_empty())
}

fn game_from(fields: &Fields, platforms: &HashSet<&str>) -> Option<LbGame> {
    let name = field(fields, "Name")?;
    let platform = field(fields, "Platform")?;
    if !platforms.contains(platform) || name.contains("[Demo]") {
        return None;
    }
    let year = field(fields, "ReleaseYear")
        .and_then(|y| y.get(..4))
        .or_else(|| field(fields, "ReleaseDate").and_then(|d| d.get(..4)))
        .and_then(|y| y.parse::<u16>().ok());
    Some(LbGame {
        id: field(fields, "DatabaseID")?.parse().ok()?,
        name: name.to_string(),
        alt_names: vec![],
        platform: platform.to_string(),
        year,
        overview: field(fields, "Overview").map(String::from),
        genres: field(fields, "Genres")
            .map(|g| g.split(';').map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect())
            .unwrap_or_default(),
        developer: field(fields, "Developer").map(String::from),
        publisher: field(fields, "Publisher").map(String::from),
        cover: None,
        hero: None,
        logo: None,
    })
}

/// Ouvre `Metadata.zip` et vérifie qu'il contient `Metadata.xml`.
fn validate_zip(path: &Path) -> Result<(), String> {
    let file = fs::File::open(path).map_err(|e| format!("Ouverture de la base LaunchBox impossible : {e}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Base LaunchBox illisible (zip corrompu ?) : {e}"))?;
    archive
        .by_name("Metadata.xml")
        .map(|_| ())
        .map_err(|e| format!("Metadata.xml introuvable dans la base LaunchBox : {e}"))
}

/// Plateforme du catalogue LaunchBox (`Platforms.xml`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LbPlatform {
    pub name: String,
    pub year: Option<u16>,
    pub manufacturer: Option<String>,
    pub category: Option<String>,
}

/// Lit les éléments `<Platform>` de `Platforms.xml` (les `PlatformAlternateName` sont ignorés),
/// triés par nom.
pub fn parse_platforms(xml: &str) -> Vec<LbPlatform> {
    let mut reader = Reader::from_str(xml);
    let mut platforms = Vec::new();
    let mut depth = 0usize;
    let mut in_platform = false;
    let mut fields: Fields = Vec::new();
    let mut current: Option<String> = None;
    let mut text = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                depth += 1;
                let name = e.name().into_inner();
                if depth == 2 {
                    in_platform = name == "Platform";
                    fields.clear();
                } else if depth == 3 && in_platform {
                    current = Some(name.to_string());
                    text.clear();
                }
            }
            Ok(Event::End(_)) => {
                if depth == 3 {
                    if let Some(key) = current.take() {
                        fields.push((key, std::mem::take(&mut text)));
                    }
                } else if depth == 2 && in_platform {
                    in_platform = false;
                    let get = |k: &str| field(&fields, k).map(str::trim).filter(|v| !v.is_empty());
                    if let Some(name) = get("Name") {
                        let year = get("ReleaseDate").and_then(|d| d.get(..4)).and_then(|y| y.parse::<u16>().ok());
                        platforms.push(LbPlatform {
                            name: name.to_string(),
                            year,
                            manufacturer: get("Manufacturer").map(str::to_string),
                            category: get("Category").map(str::to_string),
                        });
                    }
                }
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Text(t)) if current.is_some() => text.push_str(&t.xml10_content()),
            Ok(Event::CData(c)) if current.is_some() => text.push_str(c.as_ref()),
            Ok(Event::GeneralRef(r)) if current.is_some() => match r.as_ref() {
                "amp" => text.push('&'),
                "lt" => text.push('<'),
                "gt" => text.push('>'),
                "apos" => text.push('\''),
                "quot" => text.push('"'),
                _ => {
                    if let Ok(Some(c)) = r.resolve_char_ref() {
                        text.push(c);
                    }
                }
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    platforms.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then_with(|| a.name.cmp(&b.name)));
    platforms
}

/// Lit `Platforms.xml` dans `Metadata.zip`.
pub fn read_platforms(zip_path: &Path) -> Result<Vec<LbPlatform>, String> {
    let file = fs::File::open(zip_path).map_err(|e| format!("Ouverture de la base LaunchBox impossible : {e}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Base LaunchBox illisible (zip corrompu ?) : {e}"))?;
    let mut entry = archive
        .by_name("Platforms.xml")
        .map_err(|e| format!("Platforms.xml introuvable dans la base LaunchBox : {e}"))?;
    let mut xml = String::new();
    entry.read_to_string(&mut xml).map_err(|e| format!("Lecture de Platforms.xml impossible : {e}"))?;
    Ok(parse_platforms(&xml))
}

/// Plateformes de la base, ou liste vide si elle est absente ou illisible (jamais d'erreur).
pub fn platforms_or_empty(zip_path: &Path) -> Vec<LbPlatform> {
    read_platforms(zip_path).unwrap_or_default()
}

/// Parcourt `Metadata.xml` du zip en flux et appelle `on_record` pour chaque `Game`,
/// `GameAlternateName` et `GameImage` (champs à plat).
fn stream_records(zip_path: &Path, mut on_record: impl FnMut(Record, &Fields)) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|e| format!("Ouverture de la base LaunchBox impossible : {e}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Base LaunchBox illisible (zip corrompu ?) : {e}"))?;
    let entry = archive
        .by_name("Metadata.xml")
        .map_err(|e| format!("Metadata.xml introuvable dans la base LaunchBox : {e}"))?;
    let mut reader = Reader::from_reader(BufReader::with_capacity(1 << 20, entry));

    let mut buf = Vec::new();
    let mut depth = 0usize;
    let mut record: Option<Record> = None;
    let mut fields: Fields = Vec::new();
    let mut current: Option<String> = None;
    let mut text = String::new();

    loop {
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| format!("Lecture de Metadata.xml impossible : {e}"))?;
        match event {
            Event::Start(e) => {
                depth += 1;
                let name = e.name().into_inner();
                if depth == 2 {
                    record = match name {
                        "Game" => Some(Record::Game),
                        "GameAlternateName" => Some(Record::Alt),
                        "GameImage" => Some(Record::Image),
                        _ => None,
                    };
                    fields.clear();
                } else if depth == 3 && record.is_some() {
                    current = Some(name.to_string());
                    text.clear();
                }
            }
            Event::End(_) => {
                if depth == 3 {
                    if let Some(key) = current.take() {
                        fields.push((key, std::mem::take(&mut text)));
                    }
                } else if depth == 2 {
                    if let Some(rec) = record.take() {
                        on_record(rec, &fields);
                    }
                }
                depth = depth.saturating_sub(1);
            }
            Event::Text(t) if current.is_some() => text.push_str(&t.xml10_content()),
            Event::CData(c) if current.is_some() => text.push_str(c.as_ref()),
            Event::GeneralRef(r) if current.is_some() => match r.as_ref() {
                "amp" => text.push('&'),
                "lt" => text.push('<'),
                "gt" => text.push('>'),
                "apos" => text.push('\''),
                "quot" => text.push('"'),
                _ => {
                    if let Ok(Some(c)) = r.resolve_char_ref() {
                        text.push(c);
                    }
                }
            },
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

/// Construit l'index en lisant `Metadata.xml` du zip en flux (le fichier fait des centaines de Mo).
/// Deux passes : la première garde les jeux des `platforms` ; la seconde ne retient, pour ces seuls
/// `DatabaseID`, que les noms alternatifs et la meilleure image de chaque sorte.
pub fn build_index(zip_path: &Path, platforms: &[&str]) -> Result<LbIndex, String> {
    let built_from = fs::metadata(zip_path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let wanted: HashSet<&str> = platforms.iter().copied().collect();

    let mut games: Vec<LbGame> = Vec::new();
    stream_records(zip_path, |rec, fields| {
        if rec == Record::Game {
            games.extend(game_from(fields, &wanted));
        }
    })?;

    let ids: HashSet<u64> = games.iter().map(|g| g.id).collect();
    let mut alts: HashMap<u64, Vec<String>> = HashMap::new();
    let mut best: HashMap<u64, [Best; 3]> = HashMap::new();
    stream_records(zip_path, |rec, fields| {
        let Some(id) = field(fields, "DatabaseID").and_then(|v| v.parse::<u64>().ok()) else {
            return;
        };
        if !ids.contains(&id) {
            return;
        }
        match rec {
            Record::Game => {}
            Record::Alt => {
                if let Some(n) = field(fields, "AlternateName") {
                    alts.entry(id).or_default().push(n.to_string());
                }
            }
            Record::Image => {
                if let (Some(t), Some(f)) = (field(fields, "Type"), field(fields, "FileName")) {
                    let region = field(fields, "Region");
                    let slots = best.entry(id).or_default();
                    for (slot, kind) in slots.iter_mut().zip(KINDS) {
                        consider(slot, kind, t, region, f);
                    }
                }
            }
        }
    })?;

    for g in &mut games {
        for n in alts.remove(&g.id).unwrap_or_default() {
            if n != g.name && !g.alt_names.contains(&n) {
                g.alt_names.push(n);
            }
        }
        if let Some([cover, hero, logo]) = best.remove(&g.id) {
            g.cover = cover.map(|(_, _, f)| f);
            g.hero = hero.map(|(_, _, f)| f);
            g.logo = logo.map(|(_, _, f)| f);
        }
    }
    Ok(LbIndex { built_from, platforms: platforms.iter().map(|p| p.to_string()).collect(), games })
}

/// Garantit la présence d'un `Metadata.zip` valide et récent dans `dir` : ne télécharge pas s'il a
/// moins de `max_age_days` jours et s'ouvre correctement (sinon il est supprimé puis retéléchargé).
/// Si le retéléchargement d'une base trop ancienne échoue (hors ligne), l'ancienne base valide
/// est gardée, sauf pour une mise à jour demandée (`max_age_days == 0`).
/// `on_progress(téléchargé, total)` (total = 0 si inconnu).
pub fn ensure_db(
    client: &reqwest::blocking::Client,
    dir: &Path,
    max_age_days: u64,
    on_progress: impl FnMut(u64, u64),
) -> Result<PathBuf, String> {
    ensure_db_from(client, DB_URL, dir, max_age_days, on_progress)
}

fn ensure_db_from(
    client: &reqwest::blocking::Client,
    url: &str,
    dir: &Path,
    max_age_days: u64,
    on_progress: impl FnMut(u64, u64),
) -> Result<PathBuf, String> {
    let dest = dir.join(DB_NAME);
    let max_age = Duration::from_secs(max_age_days * 24 * 3600);
    let fresh = fs::metadata(&dest)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age < max_age);
    if fresh {
        if validate_zip(&dest).is_ok() {
            return Ok(dest);
        }
        let _ = fs::remove_file(&dest);
    }
    match download_to(client, url, dir, on_progress) {
        Err(e) if max_age_days > 0 && validate_zip(&dest).is_ok() => {
            eprintln!("Base LaunchBox non mise à jour, l'ancienne est gardée : {e}");
            Ok(dest)
        }
        result => result,
    }
}

/// Télécharge `url` en flux dans `dir/Metadata.zip.part`, vérifie que c'est un zip contenant
/// `Metadata.xml`, puis le renomme en `Metadata.zip`. Le `.part` est supprimé en cas d'erreur.
fn download_to(
    client: &reqwest::blocking::Client,
    url: &str,
    dir: &Path,
    mut on_progress: impl FnMut(u64, u64),
) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| format!("Création du dossier impossible : {e}"))?;
    let dest = dir.join(DB_NAME);
    let part = dir.join(format!("{DB_NAME}.part"));
    let result = download(client, url, &part, &mut on_progress).and_then(|_| validate_zip(&part)).and_then(|_| {
        fs::rename(&part, &dest).map_err(|e| format!("Enregistrement de la base LaunchBox impossible : {e}"))
    });
    if let Err(e) = result {
        let _ = fs::remove_file(&part);
        return Err(e);
    }
    Ok(dest)
}

fn download(
    client: &reqwest::blocking::Client,
    url: &str,
    part: &Path,
    on_progress: &mut impl FnMut(u64, u64),
) -> Result<(), String> {
    // Le client partagé a un délai court : la base fait plus de 100 Mo, on le relève pour cette requête.
    let mut resp = client
        .get(url)
        .timeout(Duration::from_secs(3600))
        .send()
        .map_err(|e| format!("Téléchargement de la base LaunchBox impossible : {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Téléchargement de la base LaunchBox refusé : statut {}", resp.status().as_u16()));
    }
    let total = resp.content_length().unwrap_or(0);
    let mut out = std::io::BufWriter::new(
        fs::File::create(part).map_err(|e| format!("Création du fichier temporaire impossible : {e}"))?,
    );
    let mut chunk = vec![0u8; 64 * 1024];
    let mut done = 0u64;
    loop {
        let n = resp.read(&mut chunk).map_err(|e| format!("Téléchargement interrompu : {e}"))?;
        if n == 0 {
            break;
        }
        out.write_all(&chunk[..n]).map_err(|e| format!("Écriture impossible : {e}"))?;
        done += n as u64;
        on_progress(done, total);
    }
    out.flush().map_err(|e| format!("Écriture impossible : {e}"))?;
    if total > 0 && done != total {
        return Err("Téléchargement incomplet de la base LaunchBox".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::ImageKind;
    use std::io::Write;
    use std::path::{Path, PathBuf};

    /// Choisit l'image d'un type donné : types par ordre de préférence, puis meilleure région,
    /// puis premier rencontré. Entrées : (type, région, nom de fichier).
    fn pick_image(images: &[(String, Option<String>, String)], kind: ImageKind) -> Option<String> {
        let mut best = None;
        for (ty, region, file) in images {
            consider(&mut best, kind, ty, region.as_deref(), file);
        }
        best.map(|(_, _, f)| f)
    }

    fn img(t: &str, r: Option<&str>, f: &str) -> (String, Option<String>, String) {
        (t.into(), r.map(String::from), f.into())
    }

    #[test]
    fn pick_image_prefers_europe_then_type_order() {
        let imgs = vec![
            img("Box - Front", Some("North America"), "na.jpg"),
            img("Box - Front", Some("Europe"), "eu.png"),
            img("Box - Front - Reconstructed", Some("Europe"), "r.png"),
        ];
        assert_eq!(pick_image(&imgs, ImageKind::Cover).as_deref(), Some("eu.png"));
        assert_eq!(pick_image(&imgs[2..], ImageKind::Cover).as_deref(), Some("r.png"));

        let imgs = vec![img("Screenshot - Gameplay", None, "s.jpg"), img("Clear Logo", None, "l.png")];
        assert_eq!(pick_image(&imgs, ImageKind::Hero).as_deref(), Some("s.jpg"));
        assert_eq!(pick_image(&imgs, ImageKind::Logo).as_deref(), Some("l.png"));
        assert_eq!(pick_image(&imgs[1..], ImageKind::Hero), None);
    }

    const XML: &str = r#"<?xml version="1.0" standalone="yes"?>
<LaunchBox>
  <Platform><Name>Sony Playstation 2</Name></Platform>
  <GameImage><DatabaseID>1</DatabaseID><FileName>na.jpg</FileName><Type>Box - Front</Type><Region>North America</Region></GameImage>
  <Game><Name>Devil May Cry 3: Dante&apos;s Awakening</Name><ReleaseDate>2005-03-22T00:00:00-08:00</ReleaseDate>
    <Overview>Un &amp; jeu</Overview><DatabaseID>1</DatabaseID><Platform>Sony Playstation 2</Platform>
    <Genres>Action; Adventure</Genres><Developer>Capcom</Developer><Publisher>Capcom</Publisher></Game>
  <GameAlternateName><AlternateName>Devil May Cry 3</AlternateName><DatabaseID>1</DatabaseID><Region>World</Region></GameAlternateName>
  <GameImage><DatabaseID>1</DatabaseID><FileName>eu.png</FileName><Type>Box - Front</Type><Region>Europe</Region></GameImage>
  <GameImage><DatabaseID>1</DatabaseID><FileName>logo.png</FileName><Type>Clear Logo</Type></GameImage>
  <Game><Name>Okami</Name><DatabaseID>2</DatabaseID><Platform>Sony Playstation 2</Platform><ReleaseYear>2006</ReleaseYear></Game>
  <Game><Name>God of War III</Name><DatabaseID>3</DatabaseID><Platform>Sony Playstation 3</Platform></Game>
  <Game><Name>Foo [Demo]</Name><DatabaseID>4</DatabaseID><Platform>Sony Playstation 2</Platform></Game>
  <GameImage><DatabaseID>3</DatabaseID><FileName>x.jpg</FileName><Type>Box - Front</Type></GameImage>
</LaunchBox>"#;

    fn make_zip(dir: &Path) -> PathBuf {
        let path = dir.join("Metadata.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        zip.start_file("Metadata.xml", zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(XML.as_bytes()).unwrap();
        zip.finish().unwrap();
        path
    }

    const PLATFORMS_XML: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<LaunchBox>
  <Platform><Name>Sony Playstation 2</Name><ReleaseDate>2000-03-04T00:00:00+00:00</ReleaseDate><Manufacturer>Sony</Manufacturer><Category>Console</Category></Platform>
  <PlatformAlternateName><Name>GC</Name><Alternate>Gamecube</Alternate></PlatformAlternateName>
  <Platform><Name>Nintendo GameCube</Name><Emulated>true</Emulated><ReleaseDate>2001-11-18T00:00:00+00:00</ReleaseDate><Developer>Nintendo</Developer><Manufacturer>Nintendo</Manufacturer><Category>Console</Category></Platform>
  <Platform><Name>Arcade &amp; Co</Name></Platform>
</LaunchBox>"#;

    #[test]
    fn parse_platforms_reads_name_year_manufacturer() {
        let p = parse_platforms(PLATFORMS_XML);
        assert_eq!(p.len(), 3);
        assert_eq!(p[0], LbPlatform { name: "Arcade & Co".into(), year: None, manufacturer: None, category: None });
        assert_eq!(p[1].name, "Nintendo GameCube");
        assert_eq!(p[1].year, Some(2001));
        assert_eq!(p[1].manufacturer.as_deref(), Some("Nintendo"));
        assert_eq!(p[1].category.as_deref(), Some("Console"));
        assert_eq!(p[2].name, "Sony Playstation 2");
    }

    #[test]
    fn platforms_empty_without_zip() {
        let missing = std::env::temp_dir().join("allone-no-such-dir").join("Metadata.zip");
        assert!(read_platforms(&missing).is_err());
        assert!(platforms_or_empty(&missing).is_empty());
    }

    #[test]
    fn read_platforms_from_zip() {
        let dir = std::env::temp_dir().join(format!("allone-plat-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("Metadata.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        zip.start_file("Platforms.xml", zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(PLATFORMS_XML.as_bytes()).unwrap();
        zip.finish().unwrap();
        assert_eq!(read_platforms(&path).unwrap().len(), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn build_index_streams_fixture_zip() {
        let dir = tempfile::tempdir().unwrap();
        let zip = make_zip(dir.path());
        let idx = build_index(&zip, &["Sony Playstation 2"]).unwrap();
        assert_eq!(idx.games.len(), 2);
        let dmc = &idx.games[0];
        assert_eq!(dmc.id, 1);
        assert_eq!(dmc.name, "Devil May Cry 3: Dante's Awakening");
        assert_eq!(dmc.alt_names, vec!["Devil May Cry 3"]);
        assert_eq!(dmc.year, Some(2005));
        assert_eq!(dmc.overview.as_deref(), Some("Un & jeu"));
        assert_eq!(dmc.genres, vec!["Action", "Adventure"]);
        assert_eq!(dmc.developer.as_deref(), Some("Capcom"));
        assert_eq!(dmc.cover.as_deref(), Some("eu.png"));
        assert_eq!(dmc.logo.as_deref(), Some("logo.png"));
        assert_eq!(dmc.hero, None);
        assert_eq!(idx.games[1].name, "Okami");
        assert_eq!(idx.games[1].year, Some(2006));

        let saved = dir.path().join("index.json");
        idx.save(&saved).unwrap();
        let back = LbIndex::load(&saved).unwrap();
        assert_eq!(back.games.len(), 2);
        assert_eq!(back.platforms, vec!["Sony Playstation 2"]);
        assert!(!dir.path().join("index.json.tmp").exists());
    }

    #[test]
    fn build_index_rejects_corrupt_zip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Metadata.zip");
        std::fs::write(&path, b"PK garbage").unwrap();
        assert!(build_index(&path, &["Sony Playstation 2"]).is_err());
    }

    #[test]
    fn ensure_db_keeps_fresh_zip_without_request() {
        let dir = tempfile::tempdir().unwrap();
        let zip = make_zip(dir.path());
        let client = crate::media::http_client();
        let got = ensure_db(&client, dir.path(), 30, |_, _| panic!("aucun téléchargement attendu")).unwrap();
        assert_eq!(got, zip);
    }

    /// Serveur HTTP minimal : répond une fois avec `response` (octets bruts) puis ferme.
    fn serve_once(response: Vec<u8>) -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/Metadata.zip", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            use std::io::Read;
            let (mut s, _) = listener.accept().unwrap();
            let mut req = [0u8; 4096];
            let _ = s.read(&mut req);
            let _ = s.write_all(&response);
        });
        url
    }

    fn http(status: &str, headers: &str, body: &[u8]) -> Vec<u8> {
        let mut r = format!("HTTP/1.1 {status}\r\n{headers}Connection: close\r\n\r\n").into_bytes();
        r.extend_from_slice(body);
        r
    }

    fn test_client() -> reqwest::blocking::Client {
        reqwest::blocking::Client::builder().no_proxy().build().unwrap()
    }

    fn leftovers(dir: &Path) -> usize {
        std::fs::read_dir(dir).map(|d| d.count()).unwrap_or(0)
    }

    #[test]
    fn download_rejects_non_success_status() {
        let dir = tempfile::tempdir().unwrap();
        let url = serve_once(http("404 Not Found", "Content-Length: 0\r\n", b""));
        assert!(download_to(&test_client(), &url, dir.path(), |_, _| {}).is_err());
        assert_eq!(leftovers(dir.path()), 0);
    }

    #[test]
    fn download_rejects_truncated_body() {
        let dir = tempfile::tempdir().unwrap();
        let url = serve_once(http("200 OK", "Content-Length: 1000\r\n", b"PK partiel"));
        assert!(download_to(&test_client(), &url, dir.path(), |_, _| {}).is_err());
        assert_eq!(leftovers(dir.path()), 0);
    }

    #[test]
    fn download_rejects_invalid_zip_without_content_length() {
        let dir = tempfile::tempdir().unwrap();
        let url = serve_once(http("200 OK", "", b"ceci n'est pas un zip"));
        assert!(download_to(&test_client(), &url, dir.path(), |_, _| {}).is_err());
        assert_eq!(leftovers(dir.path()), 0);
    }

    #[test]
    fn download_keeps_valid_zip() {
        let src = tempfile::tempdir().unwrap();
        let bytes = std::fs::read(make_zip(src.path())).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let url = serve_once(http("200 OK", &format!("Content-Length: {}\r\n", bytes.len()), &bytes));
        let mut last = (0, 0);
        let got = download_to(&test_client(), &url, dir.path(), |d, t| last = (d, t)).unwrap();
        assert_eq!(got, dir.path().join("Metadata.zip"));
        assert_eq!(last, (bytes.len() as u64, bytes.len() as u64));
        assert!(build_index(&got, &["Sony Playstation 2"]).is_ok());
        assert_eq!(leftovers(dir.path()), 1);
    }

    #[test]
    fn ensure_db_keeps_stale_zip_when_download_fails() {
        let dir = tempfile::tempdir().unwrap();
        let zip = make_zip(dir.path());
        let old = SystemTime::now() - Duration::from_secs(40 * 24 * 3600);
        fs::File::options().write(true).open(&zip).unwrap().set_modified(old).unwrap();

        let url = serve_once(http("503 Service Unavailable", "Content-Length: 0\r\n", b""));
        let got = ensure_db_from(&test_client(), &url, dir.path(), 30, |_, _| {}).unwrap();
        assert_eq!(got, zip);

        // Mise à jour demandée : l'échec est signalé (l'ancienne base reste sur le disque).
        let url = serve_once(http("503 Service Unavailable", "Content-Length: 0\r\n", b""));
        assert!(ensure_db_from(&test_client(), &url, dir.path(), 0, |_, _| {}).is_err());
        assert!(zip.is_file());
    }

    /// Contrôle sur la vraie base : `ALLONE_LB_ZIP=<chemin> cargo test --release real_launchbox -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_launchbox_smoke() {
        let zip = std::env::var("ALLONE_LB_ZIP").expect("ALLONE_LB_ZIP non défini");
        let start = std::time::Instant::now();
        let idx = build_index(
            Path::new(&zip),
            &["Sony Playstation 2", "Sony Playstation 3", "Sony Playstation 4", "Sony PSP"],
        )
        .unwrap();
        println!("{} jeux en {:?}", idx.games.len(), start.elapsed());
        let get = |id: u64| idx.games.iter().find(|g| g.id == id).unwrap_or_else(|| panic!("id {id} absent"));
        for (id, name, platform) in [
            (26, "God of War", "Sony Playstation 2"),
            (88, "God of War II", "Sony Playstation 2"),
            (16, "Devil May Cry 3: Dante's Awakening", "Sony Playstation 2"),
            (4814, "Need for Speed: Most Wanted", "Sony Playstation 2"),
            (7205, "Def Jam: Fight for NY", "Sony Playstation 2"),
            (13359, "God of War: Ascension", "Sony Playstation 3"),
            (30763, "God of War III: Remastered", "Sony Playstation 4"),
        ] {
            let g = get(id);
            assert_eq!((g.name.as_str(), g.platform.as_str()), (name, platform), "id {id}");
        }
        for id in [89, 8493] {
            assert_eq!(get(id).platform, "Sony PSP", "id {id}");
        }
        let dmc = get(16);
        assert!(dmc.alt_names.iter().any(|n| n == "Devil May Cry 3"), "{:?}", dmc.alt_names);
        assert!(dmc.cover.is_some());
    }
}
