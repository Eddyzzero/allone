//! Téléchargement et enregistrement des images de jeux (jaquette, bannière, logo).

use image::imageops::FilterType;
use image::{DynamicImage, ImageEncoder};
use std::fs;
use std::path::{Path, PathBuf};

/// Type d'image : détermine la taille maximale et le format de sortie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    Cover,
    Hero,
    Logo,
}

/// Taille finale : réduit seulement (jamais d'agrandissement), proportions gardées.
/// Cover : hauteur <= 900, Hero : largeur <= 1920, Logo : largeur <= 800.
pub fn target_size(w: u32, h: u32, kind: ImageKind) -> (u32, u32) {
    let (value, max) = match kind {
        ImageKind::Cover => (h, 900),
        ImageKind::Hero => (w, 1920),
        ImageKind::Logo => (w, 800),
    };
    if value <= max {
        return (w, h);
    }
    let ratio = max as f64 / value as f64;
    let scale = |v: u32| ((v as f64 * ratio).round() as u32).max(1);
    match kind {
        ImageKind::Cover => (scale(w), max),
        _ => (max, scale(h)),
    }
}

/// Décode, réduit et écrit l'image dans `dir` (créé au besoin) via un fichier `.part`
/// puis renommage. Remplace un fichier existant.
pub fn store_image(bytes: &[u8], dir: &Path, kind: ImageKind) -> Result<PathBuf, String> {
    let img = image::load_from_memory(bytes).map_err(|e| format!("Image illisible : {e}"))?;
    let (w, h) = target_size(img.width(), img.height(), kind);
    let img = if (w, h) != (img.width(), img.height()) {
        img.resize_exact(w, h, FilterType::Lanczos3)
    } else {
        img
    };

    let name = match kind {
        ImageKind::Cover => "cover.jpg",
        ImageKind::Hero => "hero.jpg",
        ImageKind::Logo => "logo.png",
    };
    let mut out = Vec::new();
    match kind {
        ImageKind::Logo => {
            let rgba = img.to_rgba8();
            image::codecs::png::PngEncoder::new(&mut out)
                .write_image(rgba.as_raw(), rgba.width(), rgba.height(), image::ExtendedColorType::Rgba8)
                .map_err(|e| format!("Encodage PNG impossible : {e}"))?;
        }
        _ => {
            // Le JPEG ne gère pas la transparence : on la supprime.
            let rgb = DynamicImage::ImageRgb8(img.to_rgb8());
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 88)
                .encode_image(&rgb)
                .map_err(|e| format!("Encodage JPEG impossible : {e}"))?;
        }
    }

    fs::create_dir_all(dir).map_err(|e| format!("Création du dossier impossible : {e}"))?;
    let dest = dir.join(name);
    let part = dir.join(format!("{name}.part"));
    let result = fs::write(&part, &out).and_then(|_| fs::rename(&part, &dest));
    if let Err(e) = result {
        let _ = fs::remove_file(&part);
        return Err(format!("Écriture de l'image impossible : {e}"));
    }
    Ok(dest)
}

/// Client HTTP bloquant (à utiliser depuis un thread de fond, jamais le runtime async Tauri).
pub fn http_client() -> reqwest::blocking::Client {
    // La configuration est statique : un échec de construction est une erreur de programmation.
    reqwest::blocking::Client::builder()
        .user_agent(concat!("allone/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("configuration du client HTTP invalide")
}

/// Télécharge `url` ; erreur si le statut n'est pas 2xx.
pub fn fetch(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<u8>, String> {
    let resp = client
        .get(url)
        .send()
        .map_err(|e| format!("Téléchargement impossible ({url}) : {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("Téléchargement refusé ({url}) : statut {}", status.as_u16()));
    }
    resp.bytes()
        .map(|b| b.to_vec())
        .map_err(|e| format!("Lecture de la réponse impossible ({url}) : {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("allone-media-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn png_2000x1000() -> Vec<u8> {
        let mut img = RgbaImage::from_pixel(2000, 1000, Rgba([200, 50, 50, 255]));
        // Bande transparente à gauche : survit au redimensionnement.
        for y in 0..1000 {
            for x in 0..200 {
                img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
        let mut out = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    #[test]
    fn target_size_only_shrinks() {
        assert_eq!(target_size(561, 800, ImageKind::Cover), (561, 800));
        assert_eq!(target_size(1450, 2048, ImageKind::Cover), (637, 900));
        assert_eq!(target_size(3840, 2160, ImageKind::Hero), (1920, 1080));
        assert_eq!(target_size(3000, 662, ImageKind::Logo), (800, 177));
    }

    #[test]
    fn store_image_writes_resized_jpeg_and_png() {
        let dir = temp_dir("ok");
        let bytes = png_2000x1000();

        let hero = store_image(&bytes, &dir, ImageKind::Hero).unwrap();
        assert_eq!(hero.file_name().unwrap(), "hero.jpg");
        let img = image::open(&hero).unwrap();
        assert_eq!((img.width(), img.height()), (1920, 960));

        let logo = store_image(&bytes, &dir, ImageKind::Logo).unwrap();
        assert_eq!(logo.file_name().unwrap(), "logo.png");
        let img = image::open(&logo).unwrap().to_rgba8();
        assert_eq!((img.width(), img.height()), (800, 400));
        assert_eq!(img.get_pixel(0, 0)[3], 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn store_image_rejects_garbage() {
        let dir = temp_dir("garbage");
        assert!(store_image(b"not an image", &dir, ImageKind::Cover).is_err());
        let count = std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0);
        assert_eq!(count, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
