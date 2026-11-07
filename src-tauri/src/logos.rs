//! Logos de consoles : recherche sur Wikimedia Commons, validation et enregistrement.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Taille maximale d'un logo (2 Mo).
pub const MAX_LOGO_BYTES: usize = 2 * 1024 * 1024;
/// Extensions de logo acceptées.
const EXTENSIONS: [&str; 5] = ["svg", "png", "jpg", "jpeg", "webp"];
const COMMONS_API: &str = "https://commons.wikimedia.org/w/api.php";

/// Image candidate trouvée sur Wikimedia Commons.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogoCandidate {
    pub title: String,
    pub thumb_url: String,
    pub url: String,
    pub width: u32,
    pub height: u32,
    /// Taille du fichier d'origine, en octets.
    pub size: u64,
}

#[derive(Deserialize)]
struct CommonsResponse {
    query: Option<CommonsQuery>,
}

#[derive(Deserialize)]
struct CommonsQuery {
    #[serde(default)]
    pages: std::collections::HashMap<String, CommonsPage>,
}

#[derive(Deserialize)]
struct CommonsPage {
    #[serde(default)]
    title: String,
    #[serde(default)]
    index: u32,
    #[serde(default)]
    imageinfo: Vec<CommonsInfo>,
}

#[derive(Deserialize)]
struct CommonsInfo {
    url: Option<String>,
    thumburl: Option<String>,
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
    #[serde(default)]
    mime: String,
    #[serde(default)]
    size: u64,
}

/// Lit la réponse de l'API Commons : ne garde que les images (SVG, PNG, JPEG, WebP),
/// dans l'ordre de pertinence de la recherche. JSON invalide → liste vide.
pub fn parse_commons(json: &str) -> Vec<LogoCandidate> {
    let Ok(resp) = serde_json::from_str::<CommonsResponse>(json) else { return Vec::new() };
    let mut pages: Vec<CommonsPage> = resp.query.map(|q| q.pages.into_values().collect()).unwrap_or_default();
    pages.sort_by_key(|p| p.index);
    pages
        .into_iter()
        .filter_map(|p| {
            let info = p.imageinfo.into_iter().next()?;
            if !matches!(info.mime.as_str(), "image/svg+xml" | "image/png" | "image/jpeg" | "image/webp") {
                return None;
            }
            // Trop lourd : save_logo le refuserait, autant ne pas le proposer.
            if info.size > MAX_LOGO_BYTES as u64 {
                return None;
            }
            let url = info.url?;
            let thumb_url = info.thumburl.unwrap_or_else(|| url.clone());
            Some(LogoCandidate { title: p.title, thumb_url, url, width: info.width, height: info.height, size: info.size })
        })
        .collect()
}

/// Client HTTP pour Wikimedia : User-Agent descriptif (politique d'usage de Wikimedia), sans contact personnel.
pub fn client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent(concat!(
            "allone/",
            env!("CARGO_PKG_VERSION"),
            " (jeu de bibliothèque de bureau; https://www.mediawiki.org/wiki/API:Etiquette)"
        ))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("configuration du client HTTP invalide")
}

/// Cherche des logos « <query> logo » sur Wikimedia Commons.
pub fn search(client: &reqwest::blocking::Client, query: &str) -> Result<Vec<LogoCandidate>, String> {
    let search = format!("{query} logo");
    let resp = client
        .get(COMMONS_API)
        .query(&[
            ("action", "query"),
            ("generator", "search"),
            ("gsrnamespace", "6"),
            ("gsrsearch", search.as_str()),
            ("gsrlimit", "12"),
            ("prop", "imageinfo"),
            ("iiprop", "url|size|mime"),
            ("iiurlwidth", "320"),
            ("format", "json"),
        ])
        .send()
        .map_err(|e| format!("Recherche de logos impossible : {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Recherche de logos refusée : statut {}", resp.status().as_u16()));
    }
    let body = resp.text().map_err(|e| format!("Réponse illisible : {e}"))?;
    Ok(parse_commons(&body))
}

/// Une URL distante n'est acceptée que si elle est en https (port par défaut) sur `upload.wikimedia.org`.
pub fn allowed_url(url: &str) -> bool {
    reqwest::Url::parse(url)
        .map(|u| u.scheme() == "https" && u.host_str() == Some("upload.wikimedia.org") && u.port().is_none())
        .unwrap_or(false)
}

/// Vrai si la source est une URL http(s) (sinon c'est un chemin de fichier local).
pub fn is_http_url(source: &str) -> bool {
    reqwest::Url::parse(source).map(|u| matches!(u.scheme(), "http" | "https")).unwrap_or(false)
}

/// Télécharge un logo ; refuse une redirection vers une adresse non autorisée.
pub fn fetch_checked(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<u8>, String> {
    let resp = client.get(url).send().map_err(|e| format!("Téléchargement du logo impossible : {e}"))?;
    if !allowed_url(resp.url().as_str()) {
        return Err("Le logo a été redirigé vers une adresse non autorisée.".into());
    }
    if !resp.status().is_success() {
        return Err(format!("Téléchargement du logo refusé : statut {}", resp.status().as_u16()));
    }
    resp.bytes().map(|b| b.to_vec()).map_err(|e| format!("Lecture du logo impossible : {e}"))
}

/// Extension en minuscules d'une URL ou d'un chemin, sans query string ni fragment.
pub fn ext_from(source: &str) -> Option<String> {
    let path = source.split(['?', '#']).next().unwrap_or("");
    let name = path.rsplit(['/', '\\']).next().unwrap_or("");
    let (stem, ext) = name.rsplit_once('.')?;
    if stem.is_empty() || ext.is_empty() || !ext.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    Some(ext.to_ascii_lowercase())
}

/// Vrai si le texte (déjà en minuscules) contient un attribut `on…=` (gestionnaire d'événement).
fn has_event_handler(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    while i + 2 < b.len() {
        let boundary = i == 0 || matches!(b[i - 1], b' ' | b'\t' | b'\r' | b'\n' | b'"' | b'\'' | b'/');
        if boundary && b[i] == b'o' && b[i + 1] == b'n' {
            let mut j = i + 2;
            while j < b.len() && b[j].is_ascii_lowercase() {
                j += 1;
            }
            if j > i + 2 {
                while j < b.len() && b[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j < b.len() && b[j] == b'=' {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

/// Vérifie extension, taille et contenu d'un logo avant de l'enregistrer.
pub fn validate_logo(bytes: &[u8], ext: &str) -> Result<(), String> {
    let ext = ext.to_ascii_lowercase();
    if !EXTENSIONS.contains(&ext.as_str()) {
        return Err(format!("Format de logo non pris en charge : .{ext} (svg, png, jpg ou webp)."));
    }
    if bytes.is_empty() {
        return Err("Le logo est vide.".into());
    }
    if bytes.len() > MAX_LOGO_BYTES {
        return Err("Le logo dépasse 2 Mo.".into());
    }
    if ext == "svg" {
        let text = String::from_utf8_lossy(bytes).to_lowercase();
        if text.contains("<script")
            || text.contains("javascript:")
            || text.contains("<foreignobject")
            || has_event_handler(&text)
        {
            return Err("Ce SVG contient du code actif : il est refusé.".into());
        }
    }
    Ok(())
}

/// Écrit le logo dans `dir/<system_id>.<ext>` (fichier `.part` puis renommage) et supprime
/// l'ancien logo de ce système, quelle que soit son extension.
pub fn store_logo(bytes: &[u8], ext: &str, dir: &Path, system_id: &str) -> Result<PathBuf, String> {
    let ext = ext.to_ascii_lowercase();
    if system_id.is_empty() || system_id.contains(['/', '\\', ':']) || system_id.contains("..") {
        return Err("Identifiant de console invalide.".into());
    }
    fs::create_dir_all(dir).map_err(|e| format!("Création du dossier des logos impossible : {e}"))?;
    let dest = dir.join(format!("{system_id}.{ext}"));
    let part = dir.join(format!("{system_id}.{ext}.part"));
    if let Err(e) = fs::write(&part, bytes).and_then(|_| fs::rename(&part, &dest)) {
        let _ = fs::remove_file(&part);
        return Err(format!("Écriture du logo impossible : {e}"));
    }
    for other in EXTENSIONS.iter().filter(|e| **e != ext) {
        let _ = fs::remove_file(dir.join(format!("{system_id}.{other}")));
    }
    Ok(dest)
}

fn valid_system_id(system_id: &str) -> bool {
    !(system_id.is_empty() || system_id.contains(['/', '\\', ':']) || system_id.contains(".."))
}

/// Supprime les fichiers `dir/<system_id>.<ext>` du logo d'une console ; sans effet s'il n'y en a pas.
pub fn delete_logo_files(dir: &Path, system_id: &str) -> Result<(), String> {
    if !valid_system_id(system_id) {
        return Err("Identifiant de console invalide.".into());
    }
    for ext in EXTENSIONS {
        let _ = fs::remove_file(dir.join(format!("{system_id}.{ext}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("allone-logos-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    const COMMONS: &str = r#"{"batchcomplete":"","query":{"pages":{
      "2":{"pageid":2,"ns":6,"title":"File:GC doc.pdf","index":3,"imageinfo":[{"size":10,"width":0,"height":0,
        "thumburl":"https://thumb.wikimedia.org/x/page1-320px-doc.pdf.jpg","url":"https://upload.wikimedia.org/x/doc.pdf?utm_source=a","mime":"application/pdf"}]},
      "3":{"pageid":3,"ns":6,"title":"File:GC symbol.png","index":2,"imageinfo":[{"size":99,"width":400,"height":300,
        "thumburl":"https://thumb.wikimedia.org/x/320px-symbol.png?utm_source=a","url":"https://upload.wikimedia.org/x/symbol.png?utm_source=a","mime":"image/png"}]},
      "4":{"pageid":4,"ns":6,"title":"File:GC huge.png","index":4,"imageinfo":[{"size":3000000,"width":4000,"height":3000,
        "thumburl":"https://thumb.wikimedia.org/x/320px-huge.png","url":"https://upload.wikimedia.org/x/huge.png","mime":"image/png"}]},
      "1":{"pageid":1,"ns":6,"title":"File:GC logo.svg","index":1,"imageinfo":[{"size":7888,"width":512,"height":164,
        "thumburl":"https://thumb.wikimedia.org/x/330px-logo.svg.png?utm_source=a","url":"https://upload.wikimedia.org/x/logo.svg?utm_source=a","mime":"image/svg+xml"}]}
    }}}"#;

    #[test]
    fn parse_commons_keeps_images_only() {
        let found = parse_commons(COMMONS);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].title, "File:GC logo.svg");
        assert_eq!(found[0].thumb_url, "https://thumb.wikimedia.org/x/330px-logo.svg.png?utm_source=a");
        assert_eq!(found[0].url, "https://upload.wikimedia.org/x/logo.svg?utm_source=a");
        assert_eq!((found[0].width, found[0].height), (512, 164));
        assert_eq!(found[0].size, 7888);
        assert!(found.iter().all(|c| c.title != "File:GC huge.png"), "plus de 2 Mo : masqué");
        assert_eq!(found[1].title, "File:GC symbol.png");
        assert!(parse_commons("pas du json").is_empty());
        assert!(parse_commons(r#"{"batchcomplete":""}"#).is_empty());
    }

    #[test]
    fn ext_from_strips_query_and_lowercases() {
        assert_eq!(ext_from("https://upload.wikimedia.org/a/b/Logo.SVG?utm_source=x&y=1").as_deref(), Some("svg"));
        assert_eq!(ext_from(r"C:\logos\gc.png").as_deref(), Some("png"));
        assert_eq!(ext_from("https://x.org/noext"), None);
        assert_eq!(ext_from("https://x.org/a.b/noext"), None);
    }

    #[test]
    fn rejects_svg_with_script() {
        assert!(validate_logo(b"<svg><script>alert(1)</script></svg>", "svg").is_err());
        assert!(validate_logo(b"<svg><a href=\"JavaScript:alert(1)\"/></svg>", "svg").is_err());
        assert!(validate_logo(b"<svg><foreignObject/></svg>", "svg").is_err());
    }

    #[test]
    fn rejects_svg_with_onload() {
        assert!(validate_logo(b"<svg onload=\"x()\"></svg>", "svg").is_err());
        assert!(validate_logo(b"<svg ONCLICK = 'x()'></svg>", "svg").is_err());
    }

    #[test]
    fn accepts_clean_svg_and_png() {
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" content=\"a\" version=\"1.1\"><path d=\"M0 0\"/></svg>";
        assert!(validate_logo(svg, "svg").is_ok());
        assert!(validate_logo(&[0x89, b'P', b'N', b'G', 1, 2, 3], "png").is_ok());
        assert!(validate_logo(b"x", "gif").is_err());
        assert!(validate_logo(b"", "png").is_err());
    }

    #[test]
    fn rejects_oversized_logo() {
        assert!(validate_logo(&vec![0u8; MAX_LOGO_BYTES], "png").is_ok());
        assert!(validate_logo(&vec![0u8; MAX_LOGO_BYTES + 1], "png").is_err());
    }

    #[test]
    fn rejects_non_wikimedia_url() {
        assert!(allowed_url("https://upload.wikimedia.org/wikipedia/commons/a/ae/Logo.svg?utm_source=x"));
        assert!(!allowed_url("http://upload.wikimedia.org/wikipedia/commons/a/ae/Logo.svg"));
        assert!(!allowed_url("https://evil.com/upload.wikimedia.org/x.svg"));
        assert!(!allowed_url("https://upload.wikimedia.org.evil.com/x"));
        assert!(!allowed_url("https://user@evil.com@upload.wikimedia.org.evil.com/x"));
        assert!(!allowed_url("C:\\logo.svg"));
    }

    #[test]
    fn store_logo_replaces_other_extension() {
        let dir = temp_dir("replace");
        fs::write(dir.join("gc.png"), b"old").unwrap();
        let path = store_logo(b"<svg/>", "svg", &dir, "gc").unwrap();
        assert_eq!(path, dir.join("gc.svg"));
        assert_eq!(fs::read(&path).unwrap(), b"<svg/>");
        assert!(!dir.join("gc.png").exists());
        assert!(!dir.join("gc.svg.part").exists());
        assert!(store_logo(b"x", "svg", &dir, "../evil").is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn url_rules() {
        assert!(!allowed_url("https://upload.wikimedia.org:8443/a/Logo.svg"));
        assert!(allowed_url("https://upload.wikimedia.org:443/a/Logo.svg"));
        assert!(is_http_url("https://x.org/a.png") && is_http_url("HTTP://x.org/a.png"));
        assert!(!is_http_url(r"C:\logos\httpfoo.png") && !is_http_url("httpfoo.png"));
    }

    #[test]
    fn delete_logo_files_removes_all_extensions() {
        let dir = temp_dir("delete");
        fs::write(dir.join("gc.png"), b"x").unwrap();
        fs::write(dir.join("gc.svg"), b"x").unwrap();
        fs::write(dir.join("gc2.png"), b"x").unwrap();
        delete_logo_files(&dir, "gc").unwrap();
        assert!(!dir.join("gc.png").exists() && !dir.join("gc.svg").exists());
        assert!(dir.join("gc2.png").exists());
        delete_logo_files(&dir, "gc").unwrap(); // déjà absent : ok
        assert!(delete_logo_files(&dir, "../x").is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}
