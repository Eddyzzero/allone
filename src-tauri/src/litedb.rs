//! Lecture seule des fichiers LiteDB v4 (format utilisé par Playnite pour `games.db`) :
//! on parcourt les pages de données, on recolle les documents répartis sur plusieurs
//! pages et on décode leur BSON. Pas d'index, pas d'écriture, pas de chiffrement.

use std::collections::BTreeMap;

const PAGE_SIZE: usize = 4096;
const PAGE_HEADER: usize = 25;
const PAGE_DATA: u8 = 4;
const PAGE_EXTEND: u8 = 5;
const NO_PAGE: u32 = u32::MAX;
const MAGIC: &[u8] = b"** This is a LiteDB file **";

/// Valeur BSON (seulement les types utiles ici ; les autres sont lus puis ignorés).
#[derive(Debug, Clone, PartialEq)]
pub enum Bson {
    Double(f64),
    String(String),
    Doc(Doc),
    Array(Vec<Bson>),
    Binary(Vec<u8>),
    Bool(bool),
    /// Millisecondes depuis l'époque Unix (UTC).
    DateTime(i64),
    Int32(i32),
    Int64(i64),
    Null,
    Other,
}

pub type Doc = BTreeMap<String, Bson>;

impl Bson {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Bson::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Bson::Int32(v) => Some(v as i64),
            Bson::Int64(v) => Some(v),
            Bson::Double(v) => Some(v as i64),
            _ => None,
        }
    }

    pub fn as_array(&self) -> &[Bson] {
        match self {
            Bson::Array(a) => a,
            _ => &[],
        }
    }

    pub fn as_doc(&self) -> Option<&Doc> {
        match self {
            Bson::Doc(d) => Some(d),
            _ => None,
        }
    }
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.buf.len()).ok_or("document BSON tronqué")?;
        let s = &self.buf[self.pos..end];
        self.pos = end;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn i32(&mut self) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i64(&mut self) -> Result<i64, String> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn cstring(&mut self) -> Result<String, String> {
        let rest = &self.buf[self.pos..];
        let len = rest.iter().position(|&b| b == 0).ok_or("nom de champ BSON sans fin")?;
        let s = String::from_utf8_lossy(&rest[..len]).into_owned();
        self.pos += len + 1;
        Ok(s)
    }
    fn len(&mut self) -> Result<usize, String> {
        usize::try_from(self.i32()?).map_err(|_| "longueur BSON négative".to_string())
    }
}

/// Décode un document BSON complet.
pub fn parse_bson(bytes: &[u8]) -> Result<Doc, String> {
    let mut r = Reader { buf: bytes, pos: 0 };
    Ok(read_fields(&mut r, 0)?.into_iter().collect())
}

/// Champs d'un document (ou d'un tableau), dans leur ordre d'origine.
fn read_fields(r: &mut Reader, depth: u32) -> Result<Vec<(String, Bson)>, String> {
    if depth > 64 {
        return Err("document BSON trop imbriqué".into());
    }
    let start = r.pos;
    let size = r.len()?;
    let end = start.checked_add(size).filter(|&e| e <= r.buf.len() && size >= 5).ok_or("taille de document BSON invalide")?;
    let mut fields = Vec::new();
    while r.pos < end - 1 {
        let kind = r.u8()?;
        let key = r.cstring()?;
        let value = match kind {
            0x01 => Bson::Double(f64::from_le_bytes(r.take(8)?.try_into().unwrap())),
            0x02 => {
                let len = r.len()?;
                let s = r.take(len)?;
                Bson::String(String::from_utf8_lossy(s.strip_suffix(&[0]).unwrap_or(s)).into_owned())
            }
            0x03 => Bson::Doc(read_fields(r, depth + 1)?.into_iter().collect()),
            0x04 => Bson::Array(read_fields(r, depth + 1)?.into_iter().map(|(_, v)| v).collect()),
            0x05 => {
                let len = r.len()?;
                r.u8()?; // sous-type
                Bson::Binary(r.take(len)?.to_vec())
            }
            0x07 => {
                r.take(12)?;
                Bson::Other
            }
            0x08 => Bson::Bool(r.u8()? != 0),
            0x09 => Bson::DateTime(r.i64()?),
            0x0A => Bson::Null,
            0x10 => Bson::Int32(r.i32()?),
            0x11 | 0x12 => Bson::Int64(r.i64()?),
            0x13 => {
                r.take(16)?;
                Bson::Other
            }
            0x7F | 0xFF => Bson::Other,
            k => return Err(format!("type BSON inconnu ({k:#x})")),
        };
        fields.push((key, value));
    }
    if r.pos != end - 1 || r.u8()? != 0 {
        return Err("document BSON mal terminé".into());
    }
    Ok(fields)
}

fn header(data: &[u8], page: u32) -> Option<(u8, u32, u16)> {
    let off = (page as usize).checked_mul(PAGE_SIZE)?;
    let h = data.get(off..off + PAGE_HEADER)?;
    let kind = h[4];
    let next = u32::from_le_bytes(h[9..13].try_into().unwrap());
    let count = u16::from_le_bytes(h[13..15].try_into().unwrap());
    Some((kind, next, count))
}

/// Tous les documents d'un fichier LiteDB v4 (toutes collections confondues).
pub fn documents(data: &[u8]) -> Result<Vec<Doc>, String> {
    if data.len() < PAGE_SIZE || !data[PAGE_HEADER..].starts_with(MAGIC) {
        return Err("ce n'est pas un fichier LiteDB v4".into());
    }
    let pages = data.len() / PAGE_SIZE;
    let mut docs = Vec::new();
    for page in 0..pages as u32 {
        let Some((kind, _, count)) = header(data, page) else { break };
        if kind != PAGE_DATA {
            continue;
        }
        let page_end = (page as usize + 1) * PAGE_SIZE;
        let mut pos = page as usize * PAGE_SIZE + PAGE_HEADER;
        for _ in 0..count {
            // Bloc : index (u16), page d'extension (u32), longueur (u16), données.
            let block = data.get(pos..pos + 8).filter(|_| pos + 8 <= page_end).ok_or("bloc de données tronqué")?;
            let extend = u32::from_le_bytes(block[2..6].try_into().unwrap());
            let len = u16::from_le_bytes(block[6..8].try_into().unwrap()) as usize;
            pos += 8;
            let inline = data.get(pos..pos + len).filter(|_| pos + len <= page_end).ok_or("bloc de données tronqué")?;
            pos += len;
            let bytes = if extend == NO_PAGE { inline.to_vec() } else { extended(data, extend)? };
            docs.push(parse_bson(&bytes)?);
        }
    }
    Ok(docs)
}

/// Recolle un document stocké dans une chaîne de pages d'extension.
fn extended(data: &[u8], first: u32) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut page = first;
    let mut seen = 0usize;
    while page != NO_PAGE {
        seen += 1;
        let (kind, next, count) = header(data, page).filter(|_| seen <= data.len() / PAGE_SIZE).ok_or("chaîne de pages invalide")?;
        if kind != PAGE_EXTEND || count as usize > PAGE_SIZE - PAGE_HEADER {
            return Err("chaîne de pages invalide".into());
        }
        let off = page as usize * PAGE_SIZE + PAGE_HEADER;
        out.extend_from_slice(&data[off..off + count as usize]);
        page = next;
    }
    Ok(out)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Encodeur BSON minimal pour les tests.
    pub fn bson_doc(fields: &[(&str, Vec<u8>, u8)]) -> Vec<u8> {
        let mut body = Vec::new();
        for (k, v, t) in fields {
            body.push(*t);
            body.extend_from_slice(k.as_bytes());
            body.push(0);
            body.extend_from_slice(v);
        }
        let mut out = ((body.len() + 5) as i32).to_le_bytes().to_vec();
        out.extend(body);
        out.push(0);
        out
    }

    pub fn s(v: &str) -> (Vec<u8>, u8) {
        let mut b = ((v.len() + 1) as i32).to_le_bytes().to_vec();
        b.extend_from_slice(v.as_bytes());
        b.push(0);
        (b, 0x02)
    }

    pub fn i64v(v: i64) -> (Vec<u8>, u8) {
        (v.to_le_bytes().to_vec(), 0x12)
    }

    pub fn date(ms: i64) -> (Vec<u8>, u8) {
        (ms.to_le_bytes().to_vec(), 0x09)
    }

    pub fn arr(items: Vec<(Vec<u8>, u8)>) -> (Vec<u8>, u8) {
        let keys: Vec<String> = (0..items.len()).map(|i| i.to_string()).collect();
        let fields: Vec<(&str, Vec<u8>, u8)> = keys.iter().zip(items).map(|(k, (v, t))| (k.as_str(), v, t)).collect();
        (bson_doc(&fields), 0x04)
    }

    pub fn doc(fields: Vec<(&str, (Vec<u8>, u8))>) -> (Vec<u8>, u8) {
        let f: Vec<(&str, Vec<u8>, u8)> = fields.into_iter().map(|(k, (v, t))| (k, v, t)).collect();
        (bson_doc(&f), 0x03)
    }

    fn page(kind: u8, next: u32, count: u16, body: &[u8]) -> Vec<u8> {
        let mut p = vec![0u8; PAGE_SIZE];
        p[4] = kind;
        p[5..9].copy_from_slice(&NO_PAGE.to_le_bytes());
        p[9..13].copy_from_slice(&next.to_le_bytes());
        p[13..15].copy_from_slice(&count.to_le_bytes());
        p[PAGE_HEADER..PAGE_HEADER + body.len()].copy_from_slice(body);
        p
    }

    fn block(extend: u32, data: &[u8]) -> Vec<u8> {
        let mut b = vec![0, 0];
        b.extend_from_slice(&extend.to_le_bytes());
        b.extend_from_slice(&(data.len() as u16).to_le_bytes());
        b.extend_from_slice(data);
        b
    }

    /// Fichier LiteDB v4 : en-tête, une page de données avec un document court et un
    /// document long réparti sur deux pages d'extension.
    pub fn litedb_file(short: &[u8], long: &[u8]) -> Vec<u8> {
        let mut head = page(1, NO_PAGE, 0, MAGIC);
        head[0..4].copy_from_slice(&0u32.to_le_bytes());
        let mut body = block(NO_PAGE, short);
        body.extend(block(2, &[]));
        let mut file = head;
        file.extend(page(PAGE_DATA, NO_PAGE, 2, &body));
        let cut = PAGE_SIZE - PAGE_HEADER;
        file.extend(page(PAGE_EXTEND, 3, cut as u16, &long[..cut]));
        file.extend(page(PAGE_EXTEND, NO_PAGE, (long.len() - cut) as u16, &long[cut..]));
        file
    }

    #[test]
    fn reads_inline_and_extended_documents() {
        let (short, _) = doc(vec![("Name", s("Court")), ("Playtime", i64v(42))]);
        let big = "x".repeat(5000);
        let (long, _) = doc(vec![("Name", s("Long")), ("Description", s(&big)), ("LastActivity", date(1_700_000_000_000))]);
        let docs = documents(&litedb_file(&short, &long)).unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0]["Name"].as_str(), Some("Court"));
        assert_eq!(docs[0]["Playtime"].as_i64(), Some(42));
        assert_eq!(docs[1]["Description"].as_str().map(str::len), Some(5000));
        assert_eq!(docs[1]["LastActivity"], Bson::DateTime(1_700_000_000_000));
    }

    #[test]
    fn arrays_keep_their_order() {
        let items: Vec<_> = (0..12).map(|i| s(&format!("r{i}"))).collect();
        let (bytes, _) = doc(vec![("Roms", arr(items))]);
        let d = parse_bson(&bytes).unwrap();
        let names: Vec<_> = d["Roms"].as_array().iter().filter_map(Bson::as_str).collect();
        assert_eq!(names.first(), Some(&"r0"));
        assert_eq!(names.last(), Some(&"r11"));
        assert_eq!(names[2], "r2");
    }

    #[test]
    fn rejects_other_files() {
        assert!(documents(&vec![0u8; PAGE_SIZE * 2]).is_err());
        assert!(documents(b"{}").is_err());
    }

    #[test]
    fn truncated_bson_is_an_error() {
        let (bytes, _) = doc(vec![("Name", s("Jeu"))]);
        assert!(parse_bson(&bytes[..bytes.len() - 3]).is_err());
    }
}
