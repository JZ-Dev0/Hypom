//! Heuristic detection of key:value groups in free text (no `@` required).
//!
//! A group is detected when:
//! - optional title line without `:`
//! - at least 2 consecutive lines matching `clave : valor`
//! - groups separated by blank lines
//!
//! Excludes URLs (`://`) and bare times (`14:30`).

use crate::block::{BlockKind, Closing, Layout, ParsedBlock, Row};
use crate::options::Options;
use crate::schema::{Field, FieldType, Schema};

/// A slice of free text that is either plain or an auto-detected key:value card.
#[derive(Debug, Clone)]
pub enum TextSlice {
    Plain(String),
    Pares(ParsedBlock, String), // parsed + raw
}

/// Split free text into plain / Pares slices using key:value heuristics.
pub fn split_text_with_pairs(text: &str) -> Vec<TextSlice> {
    if text.is_empty() {
        return vec![TextSlice::Plain(text.to_string())];
    }

    let lines: Vec<&str> = text.split('\n').collect();
    let groups = find_pair_groups(&lines);

    if groups.is_empty() {
        return vec![TextSlice::Plain(text.to_string())];
    }

    let mut slices = Vec::new();
    let mut cursor = 0usize;

    for group in &groups {
        if group.start_line > cursor {
            let chunk = lines[cursor..group.start_line].join("\n");
            slices.push(TextSlice::Plain(chunk));
        }

        let raw = lines[group.start_line..group.end_line].join("\n");
        let parsed = build_pares_block(group);
        slices.push(TextSlice::Pares(parsed, raw));
        cursor = group.end_line;
    }

    if cursor < lines.len() {
        let chunk = lines[cursor..].join("\n");
        slices.push(TextSlice::Plain(chunk));
    }

    let only_one = slices.len() == 1;
    slices.retain(|s| match s {
        TextSlice::Plain(t) => !t.is_empty() || only_one,
        TextSlice::Pares(_, _) => true,
    });

    if slices.is_empty() {
        vec![TextSlice::Plain(text.to_string())]
    } else {
        slices
    }
}

#[derive(Debug)]
struct PairGroup {
    start_line: usize,
    end_line: usize, // exclusive
    title: Option<String>,
    /// (key, value)
    pairs: Vec<(String, String)>,
}

fn find_pair_groups(lines: &[&str]) -> Vec<PairGroup> {
    let mut groups = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        if lines[i].trim().is_empty() {
            i += 1;
            continue;
        }

        if let Some(group) = try_parse_group(lines, i) {
            i = group.end_line;
            groups.push(group);
        } else {
            i += 1;
        }
    }

    groups
}

fn try_parse_group(lines: &[&str], start: usize) -> Option<PairGroup> {
    let mut i = start;
    let mut title: Option<String> = None;

    if i < lines.len() && !lines[i].trim().is_empty() && parse_key_value(lines[i]).is_none() {
        title = Some(lines[i].trim().to_string());
        i += 1;
        while i < lines.len() && lines[i].trim().is_empty() {
            i += 1;
        }
    }

    let mut pairs: Vec<(String, String)> = Vec::new();

    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = parse_key_value(line) {
            pairs.push((k, v));
            i += 1;
        } else {
            break;
        }
    }

    if pairs.len() < 2 {
        return None;
    }

    let looks_structured = title.is_some()
        || pairs.len() >= 3
        || pairs.iter().any(|(k, _)| {
            let lower = k.to_lowercase();
            lower.contains(' ')
                || lower.contains("servidor")
                || lower.contains("usuario")
                || lower.contains("pass")
                || lower.contains("contra")
                || lower.contains("directorio")
                || lower.contains("host")
                || lower.contains("user")
                || lower.contains("clave")
                || lower.contains("email")
                || lower.contains("correo")
        });

    if !looks_structured {
        return None;
    }

    Some(PairGroup {
        start_line: start,
        end_line: i,
        title,
        pairs,
    })
}

/// Parse `clave : valor` — rejects URLs and bare HH:MM times.
pub fn parse_key_value(line: &str) -> Option<(String, String)> {
    let line = line.trim_end();
    if line.trim().is_empty() {
        return None;
    }

    if line.trim_start().starts_with('@') {
        return None;
    }

    let bytes = line.as_bytes();
    let mut colon_pos = None;
    let mut idx = 0;
    while idx < bytes.len() {
        if bytes[idx] == b':' {
            let is_url = idx + 2 < bytes.len() && &bytes[idx..idx + 3] == b"://";
            if !is_url {
                colon_pos = Some(idx);
                break;
            }
            idx += 3;
            continue;
        }
        idx += 1;
    }

    let colon = colon_pos?;
    let key = line[..colon].trim();
    let value = line[colon + 1..].trim();

    if key.is_empty() {
        return None;
    }

    if key.chars().all(|c| c.is_ascii_digit())
        && value.chars().all(|c| c.is_ascii_digit() || c == ':')
        && key.len() <= 2
    {
        return None;
    }

    if key.starts_with('/') || key.starts_with('\\') {
        return None;
    }

    if !key.chars().any(|c| c.is_alphabetic()) {
        return None;
    }

    Some((key.to_string(), value.to_string()))
}

fn field_type_for_key(key: &str) -> FieldType {
    let lower = key.to_lowercase();
    if ["pass", "password", "contra", "contraseña", "contrasena", "clave", "token", "secret", "pin"]
        .iter()
        .any(|s| lower == *s || lower.contains(s))
    {
        FieldType::Secret
    } else if lower.contains("email") || lower.contains("correo") {
        FieldType::Email
    } else if lower.contains("url") {
        FieldType::Url
    } else {
        FieldType::Text
    }
}

fn build_pares_block(group: &PairGroup) -> ParsedBlock {
    let fields: Vec<Field> = group
        .pairs
        .iter()
        .map(|(k, _)| Field::new(k.clone(), field_type_for_key(k)))
        .collect();

    let cells: Vec<String> = group.pairs.iter().map(|(_, v)| v.clone()).collect();
    let n = cells.len();

    ParsedBlock {
        kind: BlockKind::Pares,
        title: group.title.clone(),
        schema: Schema::new(fields),
        options: Options::default(),
        layout: Layout::KeyValue,
        rows: vec![Row {
            cells,
            src_lines: 0..n,
        }],
        issues: Vec::new(),
        closed: Closing::Detected,
    }
}

/// Serialize a Pares block back to key:value text, preserving title.
pub fn serialize_pares(block: &ParsedBlock) -> String {
    let mut lines = Vec::new();
    if let Some(title) = &block.title {
        lines.push(title.clone());
    }
    let cells = block
        .rows
        .first()
        .map(|r| r.cells.as_slice())
        .unwrap_or(&[]);
    for (field, cell) in block.schema.fields.iter().zip(cells.iter()) {
        lines.push(format!("{} : {}", field.name, cell));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_key_value_basic() {
        let (k, v) = parse_key_value("servidor FTP    	: ftp.example.com").unwrap();
        assert_eq!(k, "servidor FTP");
        assert_eq!(v, "ftp.example.com");
    }

    #[test]
    fn test_reject_url_only_line() {
        assert!(parse_key_value("https://example.com").is_none());
    }

    #[test]
    fn test_reject_time() {
        assert!(parse_key_value("14:30").is_none());
    }

    #[test]
    fn test_detect_ftp_group() {
        let text = "\
FTP PSICOLOGIA
servidor FTP    	: ftp.example.com
usuario de FTP 	: user@example.com
contraseña      	: secret
directorio      	: /home/path

otra cosa libre";
        let segs = split_text_with_pairs(text);
        assert!(segs.iter().any(|s| matches!(s, TextSlice::Pares(_, _))));
        let (block, _) = segs.iter().find_map(|s| match s {
            TextSlice::Pares(b, r) => Some((b, r)),
            _ => None,
        }).unwrap();
        assert_eq!(block.title.as_deref(), Some("FTP PSICOLOGIA"));
        assert_eq!(block.schema.fields.len(), 4);
        assert_eq!(block.rows[0].cells[0], "ftp.example.com");
    }

    #[test]
    fn test_no_false_positive_note() {
        let text = "Nota: comprar pan\nOtra linea normal";
        let segs = split_text_with_pairs(text);
        assert_eq!(segs.len(), 1);
        assert!(matches!(segs[0], TextSlice::Plain(_)));
    }
}
