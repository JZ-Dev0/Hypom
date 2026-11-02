use crate::schema::{Field, FieldType, Schema};
use crate::options::Options;

/// Parsed header of a block
#[derive(Debug, Clone)]
pub struct ParsedHeader {
    pub title: Option<String>,
    pub schema: Schema,
    pub options: Options,
    pub raw: String,
}

/// Parse the rest of the line after `@tabla` or `@registro` etc.
/// Input: everything after `@command ` (no leading space)
pub fn parse_header(rest: &str) -> ParsedHeader {
    let rest = rest.trim();
    let mut pos = 0;
    let chars: Vec<char> = rest.chars().collect();

    // 1. Parse optional title in double quotes
    let title = if chars.first() == Some(&'"') {
        pos = 1;
        let mut title = String::new();
        while pos < chars.len() && chars[pos] != '"' {
            title.push(chars[pos]);
            pos += 1;
        }
        if pos < chars.len() {
            pos += 1; // skip closing quote
        }
        // Skip whitespace after title
        while pos < chars.len() && chars[pos].is_whitespace() {
            pos += 1;
        }
        Some(title)
    } else {
        None
    };

    let remaining: String = chars[pos..].iter().collect();

    // 2. Split by | to separate fields from options
    let (fields_str, options_pairs) = split_fields_and_options(&remaining);

    // 3. Parse fields
    let schema = if fields_str.trim().is_empty() {
        Schema::empty()
    } else {
        parse_fields(fields_str.trim())
    };

    // 4. Parse options
    let options = Options::parse_from_pairs(&options_pairs);

    ParsedHeader {
        title,
        schema,
        options,
        raw: rest.to_string(),
    }
}

/// Split the remaining header into fields part and options part
/// Options start after the first `|` that is not inside quotes
fn split_fields_and_options(s: &str) -> (String, Vec<(String, String)>) {
    let mut in_quotes = false;
    let mut pipe_pos = None;

    for (i, ch) in s.char_indices() {
        match ch {
            '"' => in_quotes = !in_quotes,
            '|' if !in_quotes => {
                pipe_pos = Some(i);
                break;
            }
            _ => {}
        }
    }

    match pipe_pos {
        Some(pos) => {
            let fields = s[..pos].to_string();
            let opts_str = &s[pos + 1..];
            let options = parse_options_str(opts_str);
            (fields, options)
        }
        None => (s.to_string(), Vec::new()),
    }
}

/// Parse options string: `key=value | key=value`
fn parse_options_str(s: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let mut in_quotes = false;
    let mut current = String::new();

    for ch in s.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                current.push(ch);
            }
            '|' if !in_quotes => {
                if let Some(pair) = parse_single_option(&current) {
                    results.push(pair);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if let Some(pair) = parse_single_option(&current) {
        results.push(pair);
    }

    results
}

fn parse_single_option(s: &str) -> Option<(String, String)> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let eq_pos = s.find('=')?;
    let key = s[..eq_pos].trim().to_string();
    let mut value = s[eq_pos + 1..].trim().to_string();
    // Remove surrounding quotes from value
    if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        value = value[1..value.len() - 1].to_string();
    }
    Some((key, value))
}

/// Parse field declarations: `campo1[:tipo], campo2[:tipo], ...`
fn parse_fields(s: &str) -> Schema {
    let fields: Vec<Field> = s
        .split(',')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() {
                return None;
            }
            let (name, ty) = if let Some(colon_pos) = part.find(':') {
                let name = part[..colon_pos].trim();
                let ty_str = part[colon_pos + 1..].trim();
                let ty = FieldType::from_str_opt(ty_str);
                (name.to_string(), ty)
            } else {
                (part.to_string(), None)
            };
            Some(Field::new_auto(name, ty))
        })
        .collect();

    Schema::new(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_header() {
        let h = parse_header("\"FTP oficina\" ip, user, pass:secreto");
        assert_eq!(h.title, Some("FTP oficina".to_string()));
        assert_eq!(h.schema.fields.len(), 3);
        assert_eq!(h.schema.fields[0].name, "ip");
        assert_eq!(h.schema.fields[0].ty, FieldType::Text);
        assert_eq!(h.schema.fields[2].name, "pass");
        assert_eq!(h.schema.fields[2].ty, FieldType::Secret);
    }

    #[test]
    fn test_parse_header_with_options() {
        let h = parse_header("ip, user, pass | orden=ip | copiar=ftp://{user}:{pass}@{ip}");
        assert_eq!(h.title, None);
        assert_eq!(h.schema.fields.len(), 3);
        assert!(h.options.order.is_some());
        assert!(h.options.copy_template.is_some());
    }

    #[test]
    fn test_parse_header_no_fields() {
        let h = parse_header("\"My Title\"");
        assert_eq!(h.title, Some("My Title".to_string()));
        assert_eq!(h.schema.fields.len(), 0);
    }

    #[test]
    fn test_auto_secret_detection() {
        let h = parse_header("user, password, token");
        assert_eq!(h.schema.fields[0].ty, FieldType::Text);
        assert_eq!(h.schema.fields[1].ty, FieldType::Secret);
        assert_eq!(h.schema.fields[2].ty, FieldType::Secret);
    }
}
