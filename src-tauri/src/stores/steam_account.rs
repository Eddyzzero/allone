//! Compte Steam du PC, clé d'API Steam et liste des jeux possédés (`GetOwnedGames`).
//!
//! La clé est un secret : elle vit dans son propre fichier (jamais dans `library.json`),
//! n'est envoyée qu'à `api.steampowered.com` et n'apparaît dans aucun message d'erreur.

use crate::vdf::{self, Vdf};
use serde_json::Value;
use std::path::{Path, PathBuf};

const OWNED_URL: &str = "https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/";
const KEY_FILE: &str = "steam-api-key";

/// Compte Steam utilisé sur ce PC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteamAccount {
    pub steam_id: u64,
    /// Nom affiché (`PersonaName`).
    pub name: String,
}

/// Compte du dernier utilisateur connecté : entrée `MostRecent "1"` de `loginusers.vdf`,
/// sinon celle au plus grand `timestamp`.
pub fn current_account(root: &Path) -> Option<SteamAccount> {
    let text = std::fs::read_to_string(root.join("config").join("loginusers.vdf")).ok()?;
    let v = vdf::parse(&text).ok()?;
    let field = |u: &Vdf, k: &str| u.get(k).and_then(Vdf::str).map(str::to_string);
    let users: Vec<(u64, &Vdf)> =
        v.get("users")?.entries().iter().filter_map(|(id, u)| Some((id.parse().ok()?, u))).collect();
    let (steam_id, user) = users
        .iter()
        .find(|(_, u)| field(u, "MostRecent").as_deref() == Some("1"))
        .or_else(|| {
            users.iter().max_by_key(|(_, u)| field(u, "timestamp").and_then(|t| t.parse::<u64>().ok()).unwrap_or(0))
        })?;
    Some(SteamAccount { steam_id: *steam_id, name: field(user, "PersonaName").unwrap_or_default() })
}

/// Jeu du compte d'après l'API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedGame {
    pub app_id: u32,
    pub name: String,
    pub playtime_secs: u64,
    pub last_played: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedError {
    /// Clé refusée ou mal formée (HTTP 400/401/403).
    Refused,
    /// Réponse valide mais sans aucun jeu (clé d'un autre compte, profil privé…).
    Empty,
    /// Réseau, quota, panne de Steam ou réponse illisible : on réessaiera plus tard.
    Unavailable(String),
}

/// Erreur correspondant à un statut HTTP ; `None` pour un succès.
pub fn classify_status(code: u16) -> Option<OwnedError> {
    match code {
        200..=299 => None,
        // 400 : clé mal formée (mal copiée) ; 401/403 : clé refusée.
        400 | 401 | 403 => Some(OwnedError::Refused),
        _ => Some(OwnedError::Unavailable(format!("statut {code}"))),
    }
}

pub fn parse_owned(json: &str) -> Result<Vec<OwnedGame>, OwnedError> {
    let v: Value = serde_json::from_str(json).map_err(|e| OwnedError::Unavailable(format!("réponse illisible : {e}")))?;
    let games: Vec<OwnedGame> = v["response"]["games"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|g| {
            Some(OwnedGame {
                app_id: u32::try_from(g["appid"].as_u64()?).ok()?,
                name: g["name"].as_str().unwrap_or_default().to_string(),
                playtime_secs: g["playtime_forever"].as_u64().unwrap_or(0) * 60,
                last_played: g["rtime_last_played"].as_i64().filter(|&t| t > 0),
            })
        })
        .collect();
    if games.is_empty() { Err(OwnedError::Empty) } else { Ok(games) }
}

/// Jeux du compte, jeux gratuits joués compris.
pub fn get_owned(key: &str, steam_id: u64) -> Result<Vec<OwnedGame>, OwnedError> {
    get_owned_from(OWNED_URL, key, steam_id)
}

fn get_owned_from(url: &str, key: &str, steam_id: u64) -> Result<Vec<OwnedGame>, OwnedError> {
    // La clé est dans l'URL : on ne suit aucune redirection, elle ne part que vers l'API Steam.
    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!("allone/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| OwnedError::Unavailable(e.without_url().to_string()))?;
    let resp = client
        .get(url)
        .query(&[
            ("key", key),
            ("steamid", &steam_id.to_string()),
            ("include_appinfo", "1"),
            ("include_played_free_games", "1"),
            ("format", "json"),
        ])
        .send()
        // L'URL contient la clé : on la retire du message.
        .map_err(|e| OwnedError::Unavailable(e.without_url().to_string()))?;
    if let Some(err) = classify_status(resp.status().as_u16()) {
        return Err(err);
    }
    parse_owned(&resp.text().map_err(|e| OwnedError::Unavailable(e.without_url().to_string()))?)
}

/// Clé nettoyée des espaces et retours à la ligne ; `None` si vide.
pub fn clean_key(raw: &str) -> Option<String> {
    let k = raw.trim();
    (!k.is_empty()).then(|| k.to_string())
}

fn key_path(data_dir: &Path) -> PathBuf {
    data_dir.join(KEY_FILE)
}

pub fn load_key(data_dir: &Path) -> Option<String> {
    clean_key(&std::fs::read_to_string(key_path(data_dir)).ok()?)
}

pub fn save_key(data_dir: &Path, key: &str) -> Result<(), String> {
    let key = clean_key(key).ok_or("Clé vide")?;
    std::fs::write(key_path(data_dir), key).map_err(|e| format!("Enregistrement de la clé impossible : {e}"))
}

pub fn delete_key(data_dir: &Path) -> Result<(), String> {
    match std::fs::remove_file(key_path(data_dir)) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(format!("Suppression de la clé impossible : {e}")),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Réponse `GetOwnedGames` au format documenté, avec les 11 jeux de la bibliothèque de référence.
    /// (Little Nightmares Enhanced Edition porte un identifiant fictif.)
    const OWNED_GAMES: &str = r#"{"response":{"game_count":11,"games":[
        {"appid":1938090,"name":"Call of Duty®","playtime_forever":0,"rtime_last_played":0},
        {"appid":601150,"name":"Devil May Cry 5","playtime_forever":5,"rtime_last_played":1700000000},
        {"appid":2497900,"name":"DON'T SCREAM","playtime_forever":30,"rtime_last_played":1710000000},
        {"appid":1790600,"name":"DRAGON BALL: Sparking! ZERO","playtime_forever":600,"rtime_last_played":1720000000},
        {"appid":1766060,"name":"HumanitZ","playtime_forever":45,"rtime_last_played":1715000000},
        {"appid":1426210,"name":"It Takes Two","playtime_forever":326,"rtime_last_played":1795365015},
        {"appid":424840,"name":"Little Nightmares","playtime_forever":200,"rtime_last_played":1690000000},
        {"appid":9000001,"name":"Little Nightmares Enhanced Edition","playtime_forever":0,"rtime_last_played":0},
        {"appid":860510,"name":"Little Nightmares II","playtime_forever":300,"rtime_last_played":1695000000},
        {"appid":578080,"name":"PUBG: BATTLEGROUNDS","playtime_forever":1000,"rtime_last_played":1705000000},
        {"appid":1544020,"name":"The Callisto Protocol","playtime_forever":326,"rtime_last_played":1730000000}
    ]}}"#;

    fn login_users(entries: &[(&str, &str, Option<&str>, u64)]) -> String {
        let mut s = String::from("\"users\"\n{\n");
        for (id, name, most_recent, ts) in entries {
            s += &format!("\t\"{id}\"\n\t{{\n\t\t\"AccountName\"\t\t\"acc\"\n\t\t\"PersonaName\"\t\t\"{name}\"\n");
            if let Some(m) = most_recent {
                s += &format!("\t\t\"MostRecent\"\t\t\"{m}\"\n");
            }
            s += &format!("\t\t\"timestamp\"\t\t\"{ts}\"\n\t}}\n");
        }
        s + "}\n"
    }

    fn steam_with_users(text: &str) -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("config")).unwrap();
        fs::write(tmp.path().join("config/loginusers.vdf"), text).unwrap();
        tmp
    }

    #[test]
    fn account_prefers_most_recent() {
        let tmp = steam_with_users(&login_users(&[
            ("76561190000000001", "ancien", Some("0"), 900),
            ("76561190000000002", "actuel", Some("1"), 100),
        ]));
        let acc = current_account(tmp.path()).unwrap();
        assert_eq!(acc, SteamAccount { steam_id: 76561190000000002, name: "actuel".into() });
    }

    #[test]
    fn account_falls_back_to_latest_timestamp() {
        let tmp = steam_with_users(&login_users(&[
            ("76561190000000001", "un", None, 100),
            ("76561190000000002", "deux", None, 200),
        ]));
        assert_eq!(current_account(tmp.path()).unwrap().steam_id, 76561190000000002);
    }

    #[test]
    fn account_missing_file_is_none() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(current_account(tmp.path()).is_none());
    }

    #[test]
    fn parse_owned_reads_fixture() {
        let games = parse_owned(OWNED_GAMES).unwrap();
        assert_eq!(games.len(), 11);
        let itt = games.iter().find(|g| g.app_id == 1426210).unwrap();
        assert_eq!(itt.name, "It Takes Two");
        let dmc = games.iter().find(|g| g.app_id == 601150).unwrap();
        assert_eq!(dmc.playtime_secs, 300);
        assert_eq!(dmc.last_played, Some(1700000000));
        let cod = games.iter().find(|g| g.app_id == 1938090).unwrap();
        assert_eq!(cod.last_played, None, "0 = jamais joué");
    }

    #[test]
    fn parse_owned_empty_response_is_empty() {
        assert_eq!(parse_owned(r#"{"response":{}}"#), Err(OwnedError::Empty));
        assert_eq!(parse_owned(r#"{"response":{"game_count":0}}"#), Err(OwnedError::Empty));
        assert!(matches!(parse_owned("pas du json"), Err(OwnedError::Unavailable(_))));
    }

    #[test]
    fn status_classification() {
        assert_eq!(classify_status(400), Some(OwnedError::Refused), "clé mal formée");
        assert_eq!(classify_status(401), Some(OwnedError::Refused));
        assert_eq!(classify_status(403), Some(OwnedError::Refused));
        for code in [429, 500, 503] {
            assert!(matches!(classify_status(code), Some(OwnedError::Unavailable(_))), "{code}");
        }
        assert_eq!(classify_status(200), None);
    }

    /// Une redirection ne doit jamais emporter la clé vers un autre serveur.
    #[test]
    fn redirect_is_not_followed() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let other = TcpListener::bind("127.0.0.1:0").unwrap();
        other.set_nonblocking(true).unwrap();
        let other_port = other.local_addr().unwrap().port();
        let api = TcpListener::bind("127.0.0.1:0").unwrap();
        let api_url = format!("http://127.0.0.1:{}/", api.local_addr().unwrap().port());
        let server = std::thread::spawn(move || {
            let (mut s, _) = api.accept().unwrap();
            let mut buf = [0u8; 2048];
            let _ = s.read(&mut buf);
            let reply = format!(
                "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{other_port}/vol\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            s.write_all(reply.as_bytes()).unwrap();
        });
        let result = get_owned_from(&api_url, "SECRET", 1);
        server.join().unwrap();
        assert!(matches!(result, Err(OwnedError::Unavailable(_))), "{result:?}");
        assert!(other.accept().is_err(), "la redirection a été suivie");
    }

    #[test]
    fn key_is_trimmed_and_round_trips() {
        assert_eq!(clean_key("  ABC\n"), Some("ABC".to_string()));
        assert_eq!(clean_key(" \n"), None);
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(load_key(tmp.path()), None);
        save_key(tmp.path(), " ABC\r\n").unwrap();
        assert_eq!(load_key(tmp.path()), Some("ABC".to_string()));
        delete_key(tmp.path()).unwrap();
        assert_eq!(load_key(tmp.path()), None);
        delete_key(tmp.path()).unwrap();
    }
}
