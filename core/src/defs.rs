//! `@def` custom block type definitions (per-note scope).

use crate::header::parse_header;
use crate::lexer::{classify_line, LineKind};
use crate::options::Options;
use crate::schema::Schema;

/// A reusable block definition from `@def name fields | options`
#[derive(Debug, Clone)]
pub struct BlockDef {
    pub name: String,
    pub schema: Schema,
    pub options: Options,
}

/// Collect all `@def` lines from a note (normalized LF text).
pub fn collect_defs(text: &str) -> Vec<BlockDef> {
    let mut defs: Vec<BlockDef> = Vec::new();
    for line in text.lines() {
        if matches!(classify_line(line), LineKind::Def) {
            if let Some(def) = parse_def_line(line) {
                // Later defs with same name override earlier ones
                if let Some(existing) = defs.iter_mut().find(|d| d.name.eq_ignore_ascii_case(&def.name)) {
                    *existing = def;
                } else {
                    defs.push(def);
                }
            }
        }
    }
    defs
}

/// Parse `@def name fields | options`
pub fn parse_def_line(line: &str) -> Option<BlockDef> {
    let rest = line.strip_prefix('@')?;
    let rest = rest.trim_start();
    // strip "def"
    let rest = rest.strip_prefix("def").or_else(|| rest.strip_prefix("DEF"))?;
    let rest = rest.trim_start();
    if rest.is_empty() {
        return None;
    }

    // First token is the type name
    let name_end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
    let name = rest[..name_end].trim().to_string();
    if name.is_empty() {
        return None;
    }

    // Built-in names cannot be overridden — still parse but caller may warn
    let after_name = rest[name_end..].trim_start();
    let header = parse_header(after_name);

    Some(BlockDef {
        name,
        schema: header.schema,
        options: header.options,
    })
}

pub fn def_names(defs: &[BlockDef]) -> Vec<String> {
    defs.iter().map(|d| d.name.to_lowercase()).collect()
}

pub fn find_def<'a>(defs: &'a [BlockDef], name: &str) -> Option<&'a BlockDef> {
    defs.iter().find(|d| d.name.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::FieldType;

    #[test]
    fn test_parse_def() {
        let def = parse_def_line(
            "@def ftp ip, user, pass:secreto | copiar=ftp://{user}:{pass}@{ip}",
        )
        .unwrap();
        assert_eq!(def.name, "ftp");
        assert_eq!(def.schema.fields.len(), 3);
        assert_eq!(def.schema.fields[2].ty, FieldType::Secret);
        assert!(def.options.copy_template.is_some());
    }

    #[test]
    fn test_collect_defs() {
        let text = "hola\n@def ftp ip, user\n@tabla x\n1\n@fin\n@ftp\na b\n@fin";
        let defs = collect_defs(text);
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "ftp");
    }
}
