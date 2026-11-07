//! Lecteur du format texte KeyValues (VDF) de Steam.

#[derive(Debug, Clone, PartialEq)]
pub enum Vdf {
    Str(String),
    Obj(Vec<(String, Vdf)>),
}

pub fn parse(text: &str) -> Result<Vdf, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut p = Parser { chars: text.chars().peekable() };
    Ok(Vdf::Obj(p.entries(true)?))
}

enum Token {
    Str(String),
    Open,
    Close,
    Eof,
}

struct Parser<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl Parser<'_> {
    fn next_token(&mut self) -> Result<Token, String> {
        loop {
            match self.chars.peek().copied() {
                None => return Ok(Token::Eof),
                Some(c) if c.is_whitespace() => {
                    self.chars.next();
                }
                Some('/') => {
                    self.chars.next();
                    if self.chars.peek() != Some(&'/') {
                        return Err("caractère '/' inattendu".into());
                    }
                    for c in self.chars.by_ref() {
                        if c == '\n' {
                            break;
                        }
                    }
                }
                Some('{') => {
                    self.chars.next();
                    return Ok(Token::Open);
                }
                Some('}') => {
                    self.chars.next();
                    return Ok(Token::Close);
                }
                Some('"') => {
                    self.chars.next();
                    return self.string().map(Token::Str);
                }
                Some(c) => return Err(format!("caractère inattendu : {c:?}")),
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        let mut out = String::new();
        loop {
            match self.chars.next() {
                None => return Err("chaîne non terminée".into()),
                Some('"') => return Ok(out),
                Some('\\') => match self.chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('\\') => out.push('\\'),
                    Some('"') => out.push('"'),
                    // Échappement inconnu : conservé tel quel (lecture tolérante).
                    Some(c) => {
                        out.push('\\');
                        out.push(c);
                    }
                    None => return Err("chaîne non terminée".into()),
                },
                Some(c) => out.push(c),
            }
        }
    }

    /// Lit les paires clé/valeur jusqu'à `}` (ou jusqu'à la fin du texte si `top`).
    fn entries(&mut self, top: bool) -> Result<Vec<(String, Vdf)>, String> {
        let mut out = Vec::new();
        loop {
            let key = match self.next_token()? {
                Token::Str(k) => k,
                Token::Close if !top => return Ok(out),
                Token::Eof if top => return Ok(out),
                Token::Eof => return Err("accolade fermante manquante".into()),
                Token::Close => return Err("accolade fermante en trop".into()),
                Token::Open => return Err("accolade ouvrante sans clé".into()),
            };
            let value = match self.next_token()? {
                Token::Str(v) => Vdf::Str(v),
                Token::Open => Vdf::Obj(self.entries(false)?),
                Token::Close | Token::Eof => return Err(format!("valeur manquante pour {key:?}")),
            };
            out.push((key, value));
        }
    }
}

impl Vdf {
    /// Première entrée portant cette clé (insensible à la casse).
    pub fn get(&self, key: &str) -> Option<&Vdf> {
        self.entries().iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v)
    }

    pub fn path(&self, keys: &[&str]) -> Option<&Vdf> {
        keys.iter().try_fold(self, |v, k| v.get(k))
    }

    pub fn str(&self) -> Option<&str> {
        match self {
            Vdf::Str(s) => Some(s),
            Vdf::Obj(_) => None,
        }
    }

    pub fn entries(&self) -> &[(String, Vdf)] {
        match self {
            Vdf::Obj(e) => e,
            Vdf::Str(_) => &[],
        }
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    pub(crate) const LIBRARYFOLDERS: &str = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"C:\\\\Program Files\\\\Jeux\"\n\t\t\"label\"\t\t\"\"\n\t\t\"apps\"\n\t\t{\n\t\t\t\"228980\"\t\t\"0\"\n\t\t\t\"1426210\"\t\t\"0\"\n\t\t}\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"D:\\\\SteamLibrary\"\n\t}\n}\n";
    pub(crate) const MANIFEST: &str = "\"AppState\"\n{\n\t\"appid\"\t\t\"1426210\"\n\t\"name\"\t\t\"It Takes Two\"\n\t\"installdir\"\t\t\"ItTakesTwo\"\n}\n";
    pub(crate) const LOCALCONFIG: &str = "\"UserLocalConfigStore\"\n{\n\t\"Software\"\n\t{\n\t\t\"Valve\"\n\t\t{\n\t\t\t\"Steam\"\n\t\t\t{\n\t\t\t\t\"apps\"\n\t\t\t\t{\n\t\t\t\t\t\"1544020\"\n\t\t\t\t\t{\n\t\t\t\t\t\t\"LastPlayed\"\t\t\"1730000000\"\n\t\t\t\t\t\t\"Playtime\"\t\t\"326\"\n\t\t\t\t\t}\n\t\t\t\t}\n\t\t\t}\n\t\t}\n\t}\n}\n";
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    #[test]
    fn reads_library_paths_with_escaped_backslashes() {
        let v = parse(LIBRARYFOLDERS).unwrap();
        let paths: Vec<_> = v
            .get("libraryfolders")
            .unwrap()
            .entries()
            .iter()
            .filter_map(|(_, e)| e.get("path").and_then(Vdf::str))
            .collect();
        assert_eq!(paths, vec!["C:\\Program Files\\Jeux", "D:\\SteamLibrary"]);
    }

    #[test]
    fn reads_manifest() {
        let v = parse(MANIFEST).unwrap();
        assert_eq!(v.path(&["AppState", "appid"]).and_then(Vdf::str), Some("1426210"));
        assert_eq!(v.path(&["AppState", "name"]).and_then(Vdf::str), Some("It Takes Two"));
    }

    #[test]
    fn keys_are_case_insensitive() {
        let v = parse(LOCALCONFIG).unwrap();
        assert_eq!(
            v.path(&["userlocalconfigstore", "software", "valve", "steam", "Apps", "1544020", "playtime"])
                .and_then(Vdf::str),
            Some("326")
        );
    }

    #[test]
    fn handles_escaped_quotes_comments_and_unicode() {
        let v = parse("// commentaire\n\"k\" \"Ünïcode \\\"x\\\"\" // fin\n").unwrap();
        assert_eq!(v.get("k").and_then(Vdf::str), Some("Ünïcode \"x\""));
    }

    #[test]
    fn decodes_backslash_newline_and_tab_escapes() {
        let v = parse(r#""k" "a\\b" "l" "x\ny\tz""#).unwrap();
        assert_eq!(v.get("k").and_then(Vdf::str), Some("a\\b"));
        assert_eq!(v.get("l").and_then(Vdf::str), Some("x\ny\tz"));
    }

    #[test]
    fn unbalanced_braces_is_an_error() {
        assert!(parse("\"a\"\n{\n\"b\" \"c\"\n").is_err());
        assert!(parse("\"a\"\n{\n}\n}\n").is_err());
    }

    #[test]
    fn skips_bom_and_keeps_duplicate_keys() {
        let v = parse("\u{feff}\"r\"\n{\n\"a\" \"1\"\n\"A\" \"2\"\n}\n").unwrap();
        let r = v.get("r").unwrap();
        assert_eq!(r.entries().len(), 2);
        assert_eq!(r.get("a").and_then(Vdf::str), Some("1"));
        assert_eq!(parse("\"a\" \"b\"").unwrap().get("a").and_then(Vdf::str), Some("b"));
    }
}
