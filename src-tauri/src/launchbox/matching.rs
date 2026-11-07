//! Correspondance des titres allone ↔ jeux LaunchBox (même plateforme uniquement).

use super::index::LbGame;
use crate::scanner::clean_title;
use std::collections::{BTreeSet, HashSet};
use std::path::Path;

/// Remplace une lettre latine accentuée par sa lettre de base (table manuelle, sans nouvelle dépendance).
fn fold_accent(c: char) -> char {
    match c {
        'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => 'a',
        'ç' => 'c',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'î' | 'ï' | 'í' | 'ì' => 'i',
        'ñ' => 'n',
        'ô' | 'ö' | 'ó' | 'ò' | 'õ' => 'o',
        'ù' | 'û' | 'ü' | 'ú' => 'u',
        'ý' | 'ÿ' => 'y',
        _ => c,
    }
}

/// Forme comparable d'un titre : minuscules, accents retirés, `&` → `and`, ponctuation → espace,
/// chiffres romains `ii`…`x` → `2`…`10` (mots entiers), « the » initial retiré.
pub fn normalize(s: &str) -> String {
    let mut spaced = String::with_capacity(s.len());
    for c in s.chars().flat_map(|c| c.to_lowercase()) {
        if c == '&' {
            spaced.push_str(" and ");
            continue;
        }
        let c = fold_accent(c);
        spaced.push(if c.is_alphanumeric() { c } else { ' ' });
    }
    let mut words: Vec<&str> = spaced
        .split_whitespace()
        .map(|w| match w {
            "ii" => "2",
            "iii" => "3",
            "iv" => "4",
            "v" => "5",
            "vi" => "6",
            "vii" => "7",
            "viii" => "8",
            "ix" => "9",
            "x" => "10",
            other => other,
        })
        .collect();
    if words.len() > 1 && words[0] == "the" {
        words.remove(0);
    }
    words.join(" ")
}

fn is_demo(g: &LbGame) -> bool {
    g.name.to_lowercase().contains("[demo]")
}

/// Coefficient de Dice sur les ensembles de mots de deux formes normalisées.
fn dice(a: &str, b: &str) -> f64 {
    let sa: HashSet<&str> = a.split(' ').filter(|w| !w.is_empty()).collect();
    let sb: HashSet<&str> = b.split(' ').filter(|w| !w.is_empty()).collect();
    if sa.is_empty() || sb.is_empty() {
        return 0.0;
    }
    let common = sa.intersection(&sb).count() as f64;
    2.0 * common / (sa.len() + sb.len()) as f64
}

/// Noms normalisés d'un jeu (principal en tête, puis alternatifs), sans les noms vides.
fn normalized_names(g: &LbGame) -> Vec<String> {
    let mut names = vec![normalize(&g.name)];
    names.extend(g.alt_names.iter().map(|a| normalize(a)));
    names.retain(|n| !n.is_empty());
    names
}

/// Ensemble des mots numériques (numéro de suite ou d'édition) d'une forme normalisée.
fn numbers(s: &str) -> BTreeSet<&str> {
    s.split(' ').filter(|w| !w.is_empty() && w.chars().all(|c| c.is_ascii_digit())).collect()
}

/// Un jeu candidat avec ses noms (principal + alternatifs) déjà normalisés ; le principal est en tête.
struct Cand<'a> {
    game: &'a LbGame,
    names: Vec<String>,
}

fn candidates<'a>(games: &'a [LbGame], platform: &str) -> Vec<Cand<'a>> {
    games
        .iter()
        .filter(|g| g.platform == platform && !is_demo(g))
        .map(|g| Cand { game: g, names: normalized_names(g) })
        .collect()
}

/// Résultat d'un indice : un jeu, aucun jeu (on peut essayer l'indice suivant) ou ambigu (on s'arrête).
enum Hint<'a> {
    Found(&'a LbGame),
    NoMatch,
    Ambiguous,
}

fn match_hint<'a>(cands: &[Cand<'a>], hint: &str) -> Hint<'a> {
    let hint = normalize(hint);
    if hint.is_empty() {
        return Hint::NoMatch;
    }
    let hint_nums = numbers(&hint);

    // 1. Nom identique ; si deux jeux différents le partagent, on ne tranche pas au hasard.
    let exact: Vec<&Cand> = cands.iter().filter(|c| c.names.iter().any(|n| *n == hint)).collect();
    let ids: BTreeSet<u64> = exact.iter().map(|c| c.game.id).collect();
    match ids.len() {
        0 => {}
        1 => return Hint::Found(exact[0].game),
        _ => return Hint::Ambiguous,
    }

    // 2. L'indice est le début de noms : un seul jeu, ou une édition de base qui précède toutes les autres.
    //    Les numéros de suite doivent être identiques (« God of War » ne prend jamais « God of War II »).
    let prefix = format!("{hint} ");
    let starting: Vec<(&Cand, &String)> = cands
        .iter()
        .filter_map(|c| {
            c.names
                .iter()
                .filter(|n| n.starts_with(&prefix) && numbers(n) == hint_nums)
                .min_by_key(|n| n.len())
                .map(|n| (c, n))
        })
        .collect();
    match starting.len() {
        0 => {}
        1 => return Hint::Found(starting[0].0.game),
        _ => {
            let base = starting.iter().find(|(c, rep)| {
                starting
                    .iter()
                    .filter(|(o, _)| !std::ptr::eq(*o, *c))
                    .all(|(_, other)| other.starts_with(&format!("{rep} ")))
            });
            if let Some((c, _)) = base {
                return Hint::Found(c.game);
            }
        }
    }

    // 3. Similarité de mots ≥ 0,8 avec une avance nette (≥ 0,1) sur le 2ᵉ jeu, mêmes numéros de suite.
    //    Epsilon : les coefficients flottants sont comparés de façon déterministe.
    const EPS: f64 = 1e-9;
    let mut scored: Vec<(f64, &LbGame)> = cands
        .iter()
        .map(|c| {
            let best = c
                .names
                .iter()
                .filter(|n| numbers(n) == hint_nums)
                .map(|n| dice(&hint, n))
                .fold(0.0, f64::max);
            (best, c.game)
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    match scored.as_slice() {
        [(best, g), ..] if *best + EPS >= 0.8 => {
            let second = scored.get(1).map_or(0.0, |s| s.0);
            if best - second + EPS >= 0.1 { Hint::Found(g) } else { Hint::NoMatch }
        }
        _ => Hint::NoMatch,
    }
}

/// Trouve le jeu LaunchBox d'une plateforme : indice 1 = titre, indice 2 = nom de fichier nettoyé.
pub fn match_game<'a>(
    games: &'a [LbGame],
    platform: &str,
    title: &str,
    file_name: Option<&str>,
) -> Option<&'a LbGame> {
    let cands = candidates(games, platform);
    match match_hint(&cands, title) {
        Hint::Found(g) => return Some(g),
        Hint::Ambiguous => return None,
        Hint::NoMatch => {}
    }
    let stem = Path::new(file_name?).file_stem()?.to_string_lossy().into_owned();
    match match_hint(&cands, &clean_title(&stem)) {
        Hint::Found(g) => Some(g),
        _ => None,
    }
}

/// Recherche par texte (accents et casse ignorés) : exact, puis préfixe, puis similarité. 20 résultats max.
pub fn search<'a>(games: &'a [LbGame], query: &str, platform: Option<&str>) -> Vec<&'a LbGame> {
    let q = normalize(query);
    if q.is_empty() {
        return Vec::new();
    }
    let q_prefix = format!("{q} ");
    let mut found: Vec<(u8, f64, &LbGame)> = Vec::new();
    for g in games.iter().filter(|g| platform.is_none_or(|p| g.platform == p) && !is_demo(g)) {
        let names = normalized_names(g);
        let sim = names.iter().map(|n| dice(&q, n)).fold(0.0, f64::max);
        let tier = if names.iter().any(|n| *n == q) {
            0
        } else if names.iter().any(|n| n.starts_with(&q_prefix)) {
            1
        } else if names.iter().any(|n| n.contains(&q)) || sim >= 0.4 {
            2
        } else {
            continue;
        };
        found.push((tier, sim, g));
    }
    found.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(b.1.total_cmp(&a.1))
            .then_with(|| a.2.name.cmp(&b.2.name))
    });
    found.into_iter().take(20).map(|(_, _, g)| g).collect()
}

/// Traduit un genre LaunchBox en français ; un genre inconnu reste tel quel.
pub fn fr_genre(g: &str) -> String {
    match g {
        "Action" => "Action",
        "Adventure" => "Aventure",
        "Racing" | "Driving" => "Course",
        "Fighting" => "Combat",
        "Sports" => "Sport",
        "Platform" => "Plateforme",
        "Shooter" => "Tir",
        "Role-Playing" => "RPG",
        "Puzzle" => "Réflexion",
        "Horror" => "Horreur",
        "Stealth" => "Infiltration",
        "Simulation" => "Simulation",
        other => other,
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PS2: &str = "Sony Playstation 2";
    const PS3: &str = "Sony Playstation 3";
    const PS4: &str = "Sony Playstation 4";
    const PSP: &str = "Sony PSP";

    fn g(id: u64, name: &str, alts: &[&str], platform: &str) -> LbGame {
        LbGame {
            id,
            name: name.into(),
            alt_names: alts.iter().map(|a| a.to_string()).collect(),
            platform: platform.into(),
            year: None,
            overview: None,
            genres: vec![],
            developer: None,
            publisher: None,
            cover: None,
            hero: None,
            logo: None,
        }
    }

    fn fixture() -> Vec<LbGame> {
        vec![
            g(30763, "God of War III: Remastered", &[], PS4),
            g(26, "God of War", &[], PS2),
            g(88, "God of War II", &["God of War 2"], PS2),
            g(13359, "God of War: Ascension", &[], PS3),
            g(34359, "God of War: Ascension - Special Edition", &[], PS3),
            g(89, "God of War: Chains of Olympus", &[], PSP),
            g(8493, "God of War: Ghost of Sparta", &[], PSP),
            g(16, "Devil May Cry 3: Dante's Awakening", &["Devil May Cry 3"], PS2),
            g(4930, "Devil May Cry 3: Special Edition", &[], PS2),
            g(4814, "Need for Speed: Most Wanted", &[], PS2),
            g(28317, "Need for Speed: Most Wanted - Black Edition", &[], PS2),
            g(218632, "Need for Speed: Most Wanted [Demo]", &[], PS2),
            g(7205, "Def Jam: Fight for NY", &[], PS2),
        ]
    }

    /// (titre allone, plateforme, fichier de la bibliothèque d'origine, id attendu)
    fn seed_cases() -> Vec<(&'static str, &'static str, &'static str, u64)> {
        vec![
            ("God of War III Remastered", PS4, "Play SpartanPort.exe", 30763),
            ("God of War", PS2, "God of War (Europe, Australia) (En,Fr,De,Es,It).iso", 26),
            ("God of War II", PS2, "God of War II (Europe, Australia) (En,Fr,De,Es,It,Ru).iso", 88),
            ("God of War: Ascension", PS3, "EBOOT.BIN", 13359),
            ("God of War: Chains of Olympus", PSP, "God of War - Chains of Olympus (Europe Australia) (EnFrDeEsIt).chd", 89),
            ("God of War: Ghost of Sparta", PSP, "God of War - Ghost of Sparta (Europe Australia) (EnFrDeEsIt).chd", 8493),
            ("Devil May Cry 3", PS2, "Devil May Cry 3 - Dante's Awakening (Europe) (En,Fr,De,Es,It).iso", 16),
            ("Need for Speed: Most Wanted", PS2, "Need for Speed - Most Wanted (Europe) (En,Fr,De,Es,It,Nl,Sv).iso", 4814),
            ("Def Jam: Fight for NY", PS2, "Def Jam - Fight for NY (Europe, Australia) (En,Fr).iso", 7205),
        ]
    }

    #[test]
    fn expected_matches() {
        let games = fixture();
        for (title, platform, file, id) in seed_cases() {
            let m = match_game(&games, platform, title, Some(file));
            assert_eq!(m.map(|x| x.id), Some(id), "{title}");
        }
    }

    #[test]
    fn filename_rescues_unknown_title() {
        let games = fixture();
        let file = "Devil May Cry 3 - Dante's Awakening (Europe) (En,Fr,De,Es,It).iso";
        assert_eq!(match_game(&games, PS2, "DMC3", Some(file)).map(|x| x.id), Some(16));
    }

    #[test]
    fn base_edition_wins_prefix_tie() {
        let games = vec![
            g(1, "Foo: Bar", &[], PS2),
            g(2, "Foo: Bar - Special Edition", &[], PS2),
        ];
        assert_eq!(match_game(&games, PS2, "Foo", None).map(|x| x.id), Some(1));
    }

    #[test]
    fn ambiguous_editions_without_base_give_no_match() {
        let games = vec![g(1, "Foo: Red", &[], PS2), g(2, "Foo: Blue", &[], PS2)];
        assert!(match_game(&games, PS2, "Foo", None).is_none());
    }

    #[test]
    fn unknown_title_gives_no_match() {
        assert!(match_game(&fixture(), PS2, "Zzz Inexistant", None).is_none());
    }

    #[test]
    fn other_platform_is_never_matched() {
        assert!(match_game(&fixture(), PSP, "God of War", None).is_none());
    }

    #[test]
    fn demo_is_ignored() {
        let games = vec![g(1, "Foo [Demo]", &[], PS2)];
        assert!(match_game(&games, PS2, "Foo", None).is_none());
    }

    #[test]
    fn similarity_needs_a_clear_lead() {
        let games = vec![g(1, "Super Mega Racing Game Deluxe", &[], PS2)];
        assert_eq!(match_game(&games, PS2, "Super Mega Racing Games Deluxe", None).map(|x| x.id), Some(1));
        let two = vec![g(1, "Super Mega Racing Game Deluxe", &[], PS2), g(2, "Super Mega Racing Games Deluxe", &[], PS2)];
        assert_eq!(match_game(&two, PS2, "Super Mega Racing Gamez Deluxe", None), None);
    }

    #[test]
    fn sequel_number_mismatch_gives_no_match() {
        let one = vec![g(1, "God of War", &[], PS2)];
        assert!(match_game(&one, PS2, "God of War III", None).is_none());
        let nfs = vec![g(1, "Need for Speed: Most Wanted", &[], PS2)];
        assert!(match_game(&nfs, PS2, "Need for Speed: Most Wanted 2", None).is_none());
        let two = vec![g(88, "God of War II", &[], PS2)];
        assert!(match_game(&two, PS2, "God of War", None).is_none());
    }

    #[test]
    fn duplicate_exact_names_give_no_match() {
        let games = vec![g(1, "Foo", &[], PS2), g(2, "Foo", &[], PS2)];
        assert!(match_game(&games, PS2, "Foo", None).is_none());
    }

    #[test]
    fn ambiguous_exact_with_longer_edition_gives_no_match() {
        let games = vec![g(1, "Foo", &[], PS2), g(2, "Foo", &[], PS2), g(3, "Foo Deluxe", &[], PS2)];
        assert!(match_game(&games, PS2, "Foo", None).is_none());
        assert!(match_game(&games, PS2, "Foo", Some("Foo Deluxe.iso")).is_none());
    }

    #[test]
    fn below_threshold_gives_no_match() {
        let games = vec![g(1, "Super Mega Racing Game", &[], PS2)];
        assert!(match_game(&games, PS2, "Super Mega Racing Games", None).is_none());
    }

    #[test]
    fn lead_of_exactly_point_one_is_accepted() {
        // Dice 0.9 contre 0.8 : l'écart flottant vaut 0,0999…98, accepté grâce à l'epsilon.
        let games = vec![
            g(1, "alpha bravo charlie delta echo foxtrot golf hotel india kilo", &[], PS2),
            g(2, "alpha bravo charlie delta echo foxtrot golf hotel lima mike", &[], PS2),
        ];
        let hint = "alpha bravo charlie delta echo foxtrot golf hotel india juliet";
        assert_eq!(match_game(&games, PS2, hint, None).map(|x| x.id), Some(1));
    }

    #[test]
    fn normalize_roman_and_punctuation() {
        assert_eq!(normalize("God of War III: Remastered"), normalize("God of War 3 Remastered"));
        assert_eq!(normalize("The Éléphant & Co"), "elephant and co");
        assert_eq!(normalize("Final Fantasy X"), "final fantasy 10");
    }

    #[test]
    fn search_orders_exact_then_prefix_and_limits() {
        let games = fixture();
        let r = search(&games, "god of war", Some(PS2));
        assert_eq!(r.iter().map(|x| x.id).collect::<Vec<_>>(), vec![26, 88]);
        assert!(search(&games, "  ", None).is_empty());
        assert_eq!(search(&games, "DEVIL may cry", None).first().map(|x| x.id), Some(16));
        let many: Vec<LbGame> = (0..30).map(|i| g(i, &format!("Foo {i}"), &[], PS2)).collect();
        assert_eq!(search(&many, "foo", None).len(), 20);
    }

    #[test]
    fn fr_genre_translates_known_only() {
        assert_eq!(fr_genre("Role-Playing"), "RPG");
        assert_eq!(fr_genre("Driving"), "Course");
        assert_eq!(fr_genre("Trivia"), "Trivia");
    }

    /// Vérification sur la vraie base :
    /// `ALLONE_LB_ZIP=...\Metadata.zip cargo test --release real_matching_smoke -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_matching_smoke() {
        let zip = std::env::var("ALLONE_LB_ZIP").expect("ALLONE_LB_ZIP");
        let index = crate::launchbox::index::build_index(Path::new(&zip), &[PS2, PS3, PS4, PSP]).unwrap();
        for (title, platform, file, id) in seed_cases() {
            let m = match_game(&index.games, platform, title, Some(file));
            println!("{title} [{platform}] -> {:?}", m.map(|x| (x.id, x.name.as_str())));
            assert_eq!(m.map(|x| x.id), Some(id), "{title}");
        }
    }
}
