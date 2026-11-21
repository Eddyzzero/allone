//! Boutique Steam : rayons mis en avant, recherche et fiches de jeux, d'après les API
//! publiques de Steam (aucun compte ni clé). L'achat lui-même se fait dans Steam.

use crate::stores::steam_web::clean_html;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

const ASSETS_BASE: &str = "https://shared.akamai.steamstatic.com/store_item_assets/";
const LANGUAGE: &str = "french";
const COUNTRY: &str = "BE";
/// Descripteurs de contenu exclus : 3 = contenu sexuel réservé aux adultes,
/// 4 = nudité ou contenu sexuel fréquents.
const ADULT_DESCRIPTORS: [i64; 2] = [3, 4];
/// Le français dans `supported_languages` (énumération ELanguage de Steam).
const LANG_FRENCH: i64 = 2;
/// Catégories Steam de prise en charge des manettes.
const CONTROLLER_FULL: i64 = 28;
const CONTROLLER_PARTIAL: i64 = 18;
/// Captures gardées par jeu.
const MAX_SCREENSHOTS: usize = 8;

/// Un jeu de la boutique, prêt à afficher.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopItem {
    pub app_id: u32,
    pub name: String,
    pub description: Option<String>,
    pub developers: Vec<String>,
    /// Date de sortie (secondes Unix) ; absente si Steam n'annonce pas de jour précis.
    pub release_date: Option<i64>,
    pub coming_soon: bool,
    pub is_free: bool,
    /// Prix affiché par Steam (« 29,99€ ») ; absent si le jeu n'est pas encore en vente.
    pub price: Option<String>,
    /// Prix avant réduction, seulement en promotion.
    pub original_price: Option<String>,
    pub discount_pct: u8,
    /// Fin de la promotion (secondes Unix).
    pub discount_ends: Option<i64>,
    /// Part d'évaluations positives ; absente sans évaluation.
    pub review_pct: Option<u8>,
    pub review_count: u32,
    /// Libellé de Steam (« très positives »).
    pub review_label: Option<String>,
    /// Grande vignette (616×353).
    pub capsule: Option<String>,
    /// Petite vignette (231×87), pour les listes.
    pub thumb: Option<String>,
    /// Captures « tout public », taille d'origine.
    pub screenshots: Vec<String>,
    /// Prise en charge du français : « Voix et textes », « Textes », ou absent.
    pub french: Option<String>,
    /// Manette : « full » ou « partial ».
    pub controller: Option<String>,
    /// Steam Deck : 3 vérifié, 2 jouable, 1 non pris en charge, 0 inconnu.
    pub deck: u8,
}

/// Un rayon de la boutique (promotions, meilleures ventes…).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopShelf {
    pub id: String,
    pub label: String,
    pub items: Vec<ShopItem>,
}

/// Rayons lus dans `featuredcategories`, dans l'ordre d'affichage.
const SHELVES: [(&str, &str); 4] = [
    ("specials", "Promotions"),
    ("top_sellers", "Meilleures ventes"),
    ("new_releases", "Nouveautés"),
    ("coming_soon", "À venir"),
];

fn get_text(client: &reqwest::blocking::Client, url: &str, query: &[(&str, String)]) -> Result<String, String> {
    let resp = client
        .get(url)
        .query(query)
        .send()
        .map_err(|e| format!("Boutique Steam injoignable : {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("La boutique Steam a refusé la requête : statut {}", status.as_u16()));
    }
    resp.text().map_err(|e| format!("Réponse de la boutique Steam illisible : {e}"))
}

/// Identifiants d'applications de chaque rayon (sans doublon ni lot de jeux).
pub fn parse_featured(json: &str) -> Vec<(&'static str, &'static str, Vec<u32>)> {
    let Ok(v) = serde_json::from_str::<Value>(json) else {
        return Vec::new();
    };
    SHELVES
        .iter()
        .map(|&(key, label)| {
            let mut seen = HashSet::new();
            let ids = v[key]["items"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|it| it["type"].as_i64() == Some(0))
                .filter_map(|it| it["id"].as_u64().and_then(|i| u32::try_from(i).ok()))
                .filter(|id| seen.insert(*id))
                .collect();
            (key, label, ids)
        })
        .collect()
}

/// Identifiants des applications trouvées par `storesearch`.
pub fn parse_search(json: &str) -> Vec<u32> {
    let Ok(v) = serde_json::from_str::<Value>(json) else {
        return Vec::new();
    };
    v["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|it| it["type"].as_str() == Some("app"))
        .filter_map(|it| it["id"].as_u64().and_then(|i| u32::try_from(i).ok()))
        .collect()
}

fn as_i64(v: &Value) -> Option<i64> {
    v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

fn text(v: &Value) -> Option<String> {
    v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(String::from)
}

/// Lit un jeu de `GetItems` ; `None` pour ce qui n'est pas un jeu en vente visible
/// (bande-son, DLC, matériel, page retirée) ou pour un contenu réservé aux adultes.
fn parse_item(it: &Value) -> Option<ShopItem> {
    if it["success"].as_i64() != Some(1) || it["visible"].as_bool() == Some(false) || it["type"].as_i64() != Some(0) {
        return None;
    }
    let descriptors: Vec<i64> = it["content_descriptorids"].as_array().into_iter().flatten().filter_map(Value::as_i64).collect();
    if descriptors.iter().any(|d| ADULT_DESCRIPTORS.contains(d)) {
        return None;
    }
    let app_id = it["id"].as_u64().and_then(|i| u32::try_from(i).ok())?;
    let name = text(&it["name"])?;

    let assets = &it["assets"];
    let asset = |key: &str| -> Option<String> {
        let format = assets["asset_url_format"].as_str()?;
        let file = assets[key].as_str()?;
        Some(format!("{ASSETS_BASE}{}", format.replace("${FILENAME}", file)))
    };

    let offer = &it["best_purchase_option"];
    let discount_pct = offer["discount_pct"].as_u64().and_then(|d| u8::try_from(d).ok()).unwrap_or(0);
    let is_free = it["is_free"].as_bool() == Some(true);

    let release = &it["release"];
    let coming_soon = it["is_coming_soon"].as_bool() == Some(true) || release["is_coming_soon"].as_bool() == Some(true);
    // Pour un jeu à venir, Steam peut n'annoncer qu'un mois ou un trimestre : on n'affiche pas de jour.
    let exact_day = !coming_soon || release["coming_soon_display"].as_str().is_none_or(|d| d == "date_full");
    let release_date = as_i64(&release["steam_release_date"]).filter(|_| exact_day);

    let summary = &it["reviews"]["summary_filtered"];
    let review_count = summary["review_count"].as_u64().and_then(|c| u32::try_from(c).ok()).unwrap_or(0);

    let french = it["supported_languages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|l| l["elanguage"].as_i64() == Some(LANG_FRENCH) && l["supported"].as_bool() == Some(true))
        .map(|l| if l["full_audio"].as_bool() == Some(true) { "Voix et textes" } else { "Textes" }.to_string());

    let controller_ids: Vec<i64> = it["categories"]["controller_categoryids"].as_array().into_iter().flatten().filter_map(Value::as_i64).collect();
    let controller = if controller_ids.contains(&CONTROLLER_FULL) {
        Some("full".to_string())
    } else if controller_ids.contains(&CONTROLLER_PARTIAL) {
        Some("partial".to_string())
    } else {
        None
    };

    Some(ShopItem {
        app_id,
        name,
        description: text(&it["basic_info"]["short_description"]).map(|d| clean_html(&d)).filter(|d| !d.is_empty()),
        developers: it["basic_info"]["developers"].as_array().into_iter().flatten().filter_map(|d| text(&d["name"])).collect(),
        release_date,
        coming_soon,
        is_free,
        price: if is_free { None } else { text(&offer["formatted_final_price"]) },
        original_price: if discount_pct > 0 { text(&offer["formatted_original_price"]) } else { None },
        discount_pct,
        discount_ends: offer["active_discounts"].as_array().into_iter().flatten().filter_map(|d| as_i64(&d["discount_end_date"])).min(),
        review_pct: (review_count > 0)
            .then(|| summary["percent_positive"].as_u64().and_then(|p| u8::try_from(p).ok()))
            .flatten(),
        review_count,
        review_label: text(&summary["review_score_label"]).filter(|_| review_count > 0),
        capsule: asset("main_capsule"),
        thumb: asset("small_capsule"),
        screenshots: it["screenshots"]["all_ages_screenshots"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|s| s["filename"].as_str())
            .take(MAX_SCREENSHOTS)
            .map(|f| format!("{ASSETS_BASE}{f}"))
            .collect(),
        french,
        controller,
        deck: it["platforms"]["steam_deck_compat_category"].as_u64().and_then(|d| u8::try_from(d).ok()).unwrap_or(0),
    })
}

/// Lit une réponse `IStoreBrowseService/GetItems` ; seules les entrées affichables sont gardées.
pub fn parse_items(json: &str) -> HashMap<u32, ShopItem> {
    let Ok(v) = serde_json::from_str::<Value>(json) else {
        return HashMap::new();
    };
    v["response"]["store_items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(parse_item)
        .map(|item| (item.app_id, item))
        .collect()
}

/// Fiches des applications demandées, par lots de 50, dans l'ordre des identifiants.
pub fn items(client: &reqwest::blocking::Client, ids: &[u32]) -> Result<Vec<ShopItem>, String> {
    let mut all = HashMap::new();
    for batch in ids.chunks(50) {
        let input = json!({
            "ids": batch.iter().map(|id| json!({ "appid": id })).collect::<Vec<_>>(),
            "context": { "language": LANGUAGE, "country_code": COUNTRY },
            "data_request": {
                "include_assets": true,
                "include_release": true,
                "include_platforms": true,
                "include_basic_info": true,
                "include_reviews": true,
                "include_screenshots": true,
                "include_supported_languages": true,
                "include_categories": true,
            },
        });
        let body = get_text(
            client,
            "https://api.steampowered.com/IStoreBrowseService/GetItems/v1",
            &[("input_json", input.to_string())],
        )?;
        all.extend(parse_items(&body));
    }
    Ok(ids.iter().filter_map(|id| all.remove(id)).collect())
}

/// Rayons de la boutique (promotions, meilleures ventes, nouveautés, à venir).
pub fn featured(client: &reqwest::blocking::Client) -> Result<Vec<ShopShelf>, String> {
    let body = get_text(
        client,
        "https://store.steampowered.com/api/featuredcategories",
        &[("cc", COUNTRY.to_string()), ("l", LANGUAGE.to_string())],
    )?;
    let shelves = parse_featured(&body);
    let mut seen = HashSet::new();
    let all_ids: Vec<u32> = shelves.iter().flat_map(|(_, _, ids)| ids.iter().copied()).filter(|id| seen.insert(*id)).collect();
    let by_id: HashMap<u32, ShopItem> = items(client, &all_ids)?.into_iter().map(|i| (i.app_id, i)).collect();
    Ok(shelves
        .into_iter()
        .map(|(id, label, ids)| ShopShelf {
            id: id.to_string(),
            label: label.to_string(),
            items: ids.iter().filter_map(|i| by_id.get(i).cloned()).collect(),
        })
        .collect())
}

/// Recherche dans la boutique ; vide pour une recherche vide.
pub fn search(client: &reqwest::blocking::Client, term: &str) -> Result<Vec<ShopItem>, String> {
    let term = term.trim();
    if term.is_empty() {
        return Ok(Vec::new());
    }
    let body = get_text(
        client,
        "https://store.steampowered.com/api/storesearch/",
        &[("term", term.to_string()), ("l", LANGUAGE.to_string()), ("cc", COUNTRY.to_string())],
    )?;
    items(client, &parse_search(&body))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAME: &str = r#"{"response":{"store_items":[
      {"item_type":0,"id":2215430,"success":1,"visible":true,"name":"Ghost","type":0,"content_descriptorids":[1,2,5],
       "categories":{"controller_categoryids":[28,55]},
       "reviews":{"summary_filtered":{"review_count":66251,"percent_positive":93,"review_score_label":"très positives"}},
       "basic_info":{"short_description":"Une tempête &amp; approche.\r\n","developers":[{"name":"Sucker Punch"}]},
       "assets":{"asset_url_format":"steam/apps/2215430/${FILENAME}?t=1","main_capsule":"capsule_616x353.jpg","small_capsule":"capsule_231x87.jpg"},
       "release":{"steam_release_date":1715871540},
       "platforms":{"steam_deck_compat_category":3},
       "best_purchase_option":{"final_price_in_cents":"2999","formatted_final_price":"29,99€","formatted_original_price":"59,99€",
         "discount_pct":50,"active_discounts":[{"discount_end_date":1796148000}]},
       "screenshots":{"all_ages_screenshots":[{"filename":"steam/apps/2215430/ss_a.jpg?t=1"}]},
       "supported_languages":[{"elanguage":0,"supported":true,"full_audio":true},{"elanguage":2,"supported":true,"full_audio":true,"subtitles":true}]},
      {"item_type":0,"id":5251630,"success":1,"name":"Adulte","type":0,"is_free":true,"content_descriptorids":[1,3,4,5]},
      {"item_type":0,"id":4165910,"success":1,"name":"Machine","type":10},
      {"item_type":0,"id":5392220,"success":1,"name":"Bande-son","type":11},
      {"item_type":0,"id":9,"success":1,"visible":false,"name":"Masqué","type":0},
      {"item_type":0,"id":7,"success":1,"name":"Bientôt","type":0,"is_coming_soon":true,
       "release":{"steam_release_date":1800000000,"is_coming_soon":true,"coming_soon_display":"date_quarter"},
       "reviews":{"summary_filtered":{"review_count":0,"percent_positive":0,"review_score_label":"aucune évaluation"}}}
    ]}}"#;

    #[test]
    fn parse_items_keeps_games_and_reads_offer() {
        let items = parse_items(GAME);
        let g = &items[&2215430];
        assert_eq!(g.price.as_deref(), Some("29,99€"));
        assert_eq!(g.original_price.as_deref(), Some("59,99€"));
        assert_eq!(g.discount_pct, 50);
        assert_eq!(g.discount_ends, Some(1796148000));
        assert_eq!(g.review_pct, Some(93));
        assert_eq!(g.review_label.as_deref(), Some("très positives"));
        assert_eq!(g.description.as_deref(), Some("Une tempête & approche."));
        assert_eq!(g.developers, vec!["Sucker Punch".to_string()]);
        assert_eq!(g.french.as_deref(), Some("Voix et textes"));
        assert_eq!(g.controller.as_deref(), Some("full"));
        assert_eq!(g.deck, 3);
        assert_eq!(g.capsule.as_deref(), Some("https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/2215430/capsule_616x353.jpg?t=1"));
        assert_eq!(g.screenshots, vec!["https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/2215430/ss_a.jpg?t=1".to_string()]);
    }

    #[test]
    fn parse_items_drops_adult_hardware_soundtracks_and_hidden() {
        let items = parse_items(GAME);
        assert!(!items.contains_key(&5251630));
        assert!(!items.contains_key(&4165910));
        assert!(!items.contains_key(&5392220));
        assert!(!items.contains_key(&9));
        assert!(parse_items("pas du json").is_empty());
    }

    #[test]
    fn coming_soon_without_exact_day_has_no_date_nor_reviews() {
        let items = parse_items(GAME);
        let g = &items[&7];
        assert!(g.coming_soon);
        assert_eq!(g.release_date, None);
        assert_eq!(g.review_pct, None);
        assert_eq!(g.review_label, None);
        assert_eq!(g.price, None);
    }

    #[test]
    fn parse_featured_reads_shelves_in_order_without_duplicates_or_bundles() {
        let json = r#"{"specials":{"items":[{"id":1,"type":0},{"id":1,"type":0},{"id":50,"type":2}]},
            "top_sellers":{"items":[{"id":2,"type":0}]},"new_releases":{"items":[]}}"#;
        let shelves = parse_featured(json);
        assert_eq!(shelves.len(), 4);
        assert_eq!(shelves[0], ("specials", "Promotions", vec![1]));
        assert_eq!(shelves[1].2, vec![2]);
        assert!(shelves[3].2.is_empty());
    }

    /// Interroge la vraie boutique : `cargo test --lib steam_shop -- --ignored --nocapture`.
    #[test]
    #[ignore = "réseau"]
    fn live_featured_and_search() {
        let client = crate::media::http_client();
        for shelf in featured(&client).unwrap() {
            println!("{} : {} jeux", shelf.label, shelf.items.len());
            for it in shelf.items.iter().take(3) {
                println!("  {} · {:?} -{}% · {:?} · {} captures", it.name, it.price, it.discount_pct, it.review_label, it.screenshots.len());
            }
        }
        let found = search(&client, "hades").unwrap();
        println!("recherche « hades » : {:?}", found.iter().map(|i| &i.name).collect::<Vec<_>>());
        assert!(!found.is_empty());
    }

    #[test]
    fn parse_search_keeps_apps_only() {
        let json = r#"{"total":3,"items":[{"type":"app","id":1145350},{"type":"sub","id":5},{"type":"app","id":1145360}]}"#;
        assert_eq!(parse_search(json), vec![1145350, 1145360]);
        assert!(parse_search("{}").is_empty());
    }
}
