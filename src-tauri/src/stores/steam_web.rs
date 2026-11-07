//! Infos et images Steam : API de la boutique et cache local de la bibliothèque.

use crate::media;
use crate::stores::steam::StoreType;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const ASSETS_BASE: &str = "https://shared.akamai.steamstatic.com/store_item_assets/";

/// Type de contenu et URL complètes des images d'une application, d'après l'API de la boutique.
#[derive(Debug, Clone, PartialEq)]
pub struct SteamItem {
    pub kind: StoreType,
    pub cover: Option<String>,
    pub hero: Option<String>,
    pub logo: Option<String>,
}

/// Lit une réponse `IStoreBrowseService/GetItems` ; renvoie une entrée par application listée.
pub fn parse_items(json: &str) -> HashMap<u32, SteamItem> {
    let Ok(v) = serde_json::from_str::<Value>(json) else {
        return HashMap::new();
    };
    let mut out = HashMap::new();
    for item in v["response"]["store_items"].as_array().into_iter().flatten() {
        let Some(id) = item["id"].as_u64().and_then(|i| u32::try_from(i).ok()) else {
            continue;
        };
        let kind = match (item["success"].as_i64(), item["type"].as_i64()) {
            (Some(1), Some(0)) => StoreType::Game,
            (Some(1), _) => StoreType::Other,
            _ => StoreType::Unknown,
        };
        let assets = &item["assets"];
        let url = |file: Option<&str>| -> Option<String> {
            let format = assets["asset_url_format"].as_str()?;
            Some(format!("{ASSETS_BASE}{}", format.replace("${FILENAME}", file?)))
        };
        let cover = url(assets["library_capsule_2x"].as_str().or(assets["library_capsule"].as_str()));
        let hero = url(assets["library_hero"].as_str());
        let logo = url(Some("logo.png"));
        out.insert(id, SteamItem { kind, cover, hero, logo });
    }
    out
}

/// Interroge la boutique par lots de 50 applications. Une erreur sur un lot fait échouer l'appel.
pub fn get_items(client: &reqwest::blocking::Client, ids: &[u32]) -> Result<HashMap<u32, SteamItem>, String> {
    let mut all = HashMap::new();
    for batch in ids.chunks(50) {
        let input = json!({
            "ids": batch.iter().map(|id| json!({ "appid": id })).collect::<Vec<_>>(),
            "context": { "language": "french", "country_code": "BE" },
            "data_request": { "include_assets": true },
        });
        let resp = client
            .get("https://api.steampowered.com/IStoreBrowseService/GetItems/v1")
            .query(&[("input_json", input.to_string())])
            .send()
            .map_err(|e| format!("Requête Steam impossible : {e}"))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("Steam a refusé la requête : statut {}", status.as_u16()));
        }
        let body = resp.text().map_err(|e| format!("Réponse Steam illisible : {e}"))?;
        all.extend(parse_items(&body));
    }
    Ok(all)
}

/// Description, genre et année d'un jeu d'après la fiche boutique.
#[derive(Debug, Clone, PartialEq)]
pub struct StoreDetails {
    pub description: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u16>,
}

/// Première suite de 4 chiffres d'une date ("25 mars 2021" → 2021).
fn year_of(date: &str) -> Option<u16> {
    let bytes = date.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i - start == 4 {
                return date[start..i].parse().ok();
            }
        } else {
            i += 1;
        }
    }
    None
}

/// Lit une réponse `appdetails` ; `None` si la boutique ne connaît pas l'application.
/// Retire les balises et décode les entités HTML (nommées courantes et numériques).
fn clean_html(s: &str) -> String {
    let mut stripped = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                stripped.push(' ');
            }
            _ if !in_tag => stripped.push(c),
            _ => {}
        }
    }
    let mut out = String::with_capacity(stripped.len());
    let mut rest = stripped.as_str();
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let decoded = rest.find(';').filter(|&e| e <= 10).and_then(|e| {
            let ch = match &rest[1..e] {
                "quot" => Some('"'),
                "amp" => Some('&'),
                "apos" => Some('\''),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "nbsp" => Some(' '),
                n => n
                    .strip_prefix('#')
                    .and_then(|n| match n.strip_prefix(['x', 'X']) {
                        Some(h) => u32::from_str_radix(h, 16).ok(),
                        None => n.parse().ok(),
                    })
                    .and_then(char::from_u32),
            };
            ch.map(|ch| (ch, e))
        });
        match decoded {
            Some((ch, e)) => {
                out.push(ch);
                rest = &rest[e + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn parse_details(json: &str, app_id: u32) -> Option<StoreDetails> {
    let v: Value = serde_json::from_str(json).ok()?;
    let entry = &v[app_id.to_string()];
    if entry["success"].as_bool() != Some(true) {
        return None;
    }
    let data = &entry["data"];
    let text = |s: &Value| s.as_str().map(str::trim).filter(|s| !s.is_empty()).map(String::from);
    Some(StoreDetails {
        description: text(&data["short_description"]).map(|d| clean_html(&d)).filter(|d| !d.is_empty()),
        genre: text(&data["genres"][0]["description"]),
        year: data["release_date"]["date"].as_str().and_then(year_of),
    })
}

/// Fiche boutique d'une application (`Ok(None)` si elle est inconnue de la boutique).
pub fn app_details(client: &reqwest::blocking::Client, app_id: u32) -> Result<Option<StoreDetails>, String> {
    let url = format!("https://store.steampowered.com/api/appdetails?appids={app_id}&l=french&cc=be");
    let bytes = media::fetch(client, &url)?;
    Ok(parse_details(&String::from_utf8_lossy(&bytes), app_id))
}

/// Images déjà présentes dans le cache de la bibliothèque Steam.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SteamLocalImages {
    pub cover: Option<PathBuf>,
    pub hero: Option<PathBuf>,
    pub logo: Option<PathBuf>,
}

/// Cherche les images de `appcache/librarycache/<id>/` sous la racine Steam.
pub fn local_cache(root: &Path, app_id: u32) -> SteamLocalImages {
    let dir = root.join("appcache").join("librarycache").join(app_id.to_string());
    let find = |name: &str| Some(dir.join(name)).filter(|p| p.is_file());
    SteamLocalImages {
        cover: find("library_600x900.jpg"),
        hero: find("library_hero.jpg"),
        logo: find("logo.png"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stores::steam::StoreType;

    const FIXTURE: &str = r#"{"response":{"store_items":[{"item_type":0,"id":1426210,"success":1,"name":"It Takes Two","type":0,
 "assets":{"asset_url_format":"steam/apps/1426210/${FILENAME}?t=1777395376","library_capsule":"library_600x900.jpg",
 "library_capsule_2x":"library_600x900_2x.jpg","library_hero":"library_hero.jpg"}}]}}"#;

    #[test]
    fn parse_items_builds_cover_hero_logo_urls() {
        let items = parse_items(FIXTURE);
        let it = &items[&1426210];
        assert_eq!(it.kind, StoreType::Game);
        let base = "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/1426210/";
        assert_eq!(it.cover.as_deref(), Some(format!("{base}library_600x900_2x.jpg?t=1777395376").as_str()));
        assert_eq!(it.hero.as_deref(), Some(format!("{base}library_hero.jpg?t=1777395376").as_str()));
        assert_eq!(it.logo.as_deref(), Some(format!("{base}logo.png?t=1777395376").as_str()));
    }

    #[test]
    fn parse_items_marks_unknown_and_other() {
        let json = r#"{"response":{"store_items":[{"item_type":0,"id":228980,"success":15,"visible":false,"name":""},
            {"item_type":0,"id":5,"success":1,"type":4}]}}"#;
        let items = parse_items(json);
        assert_eq!(items[&228980].kind, StoreType::Unknown);
        assert_eq!(items[&228980].cover, None);
        assert_eq!(items[&5].kind, StoreType::Other);
        assert!(parse_items("pas du json").is_empty());
    }

    #[test]
    fn parse_details_extracts_year_from_french_date() {
        let json = r#"{"1426210":{"success":true,"data":{"name":"It Takes Two","short_description":"Un jeu coop.",
            "genres":[{"id":"1","description":"Action"},{"id":"25","description":"Aventure"}],
            "release_date":{"coming_soon":false,"date":"25 mars 2021"}}}}"#;
        let d = parse_details(json, 1426210).unwrap();
        assert_eq!(d.year, Some(2021));
        assert_eq!(d.genre.as_deref(), Some("Action"));
        assert_eq!(d.description.as_deref(), Some("Un jeu coop."));
        assert!(parse_details(r#"{"1":{"success":false}}"#, 1).is_none());
    }

    #[test]
    fn parse_details_decodes_entities_and_strips_tags() {
        let json = r#"{"7":{"success":true,"data":{"short_description":"Un &quot;grand&quot; jeu &amp; plus<br>l&#39;aventure &lt;3 &apos;ok&apos; &#x41;"}}}"#;
        let d = parse_details(json, 7).unwrap();
        assert_eq!(d.description.as_deref(), Some("Un \"grand\" jeu & plus l'aventure <3 'ok' A"));
    }

    #[test]
    fn local_cache_finds_existing_files() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("appcache").join("librarycache").join("42");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("library_600x900.jpg"), b"x").unwrap();
        let c = local_cache(root.path(), 42);
        assert!(c.cover.is_some());
        assert!(c.hero.is_none());
        assert!(c.logo.is_none());
    }
}
