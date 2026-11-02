/// Expand a copy template like `ftp://{user}:{pass}@{ip}` using field values.
/// Returns the expanded string and a list of field names that were not found.
pub fn expand_template(template: &str, fields: &[(&str, &str)]) -> (String, Vec<String>) {
    let mut result = String::with_capacity(template.len());
    let mut missing = Vec::new();
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if i + 1 < chars.len() && chars[i] == '{' && chars[i + 1] == '{' {
            // Escaped opening brace
            result.push('{');
            i += 2;
        } else if i + 1 < chars.len() && chars[i] == '}' && chars[i + 1] == '}' {
            // Escaped closing brace
            result.push('}');
            i += 2;
        } else if chars[i] == '{' {
            // Find closing brace
            let start = i + 1;
            let mut end = start;
            while end < chars.len() && chars[end] != '}' {
                end += 1;
            }
            if end < chars.len() {
                let field_name: String = chars[start..end].iter().collect();
                if let Some((_, value)) = fields.iter().find(|(name, _)| name.eq_ignore_ascii_case(&field_name)) {
                    result.push_str(value);
                } else {
                    // Field not found, keep original
                    result.push('{');
                    result.push_str(&field_name);
                    result.push('}');
                    missing.push(field_name);
                }
                i = end + 1;
            } else {
                result.push('{');
                i = start;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    (result, missing)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_template() {
        let fields = vec![("user", "admin"), ("pass", "1234"), ("ip", "192.168.1.1")];
        let (result, missing) = expand_template("ftp://{user}:{pass}@{ip}", &fields);
        assert_eq!(result, "ftp://admin:1234@192.168.1.1");
        assert!(missing.is_empty());
    }

    #[test]
    fn test_missing_field() {
        let fields = vec![("user", "admin")];
        let (result, missing) = expand_template("{user}:{pass}", &fields);
        assert_eq!(result, "admin:{pass}");
        assert_eq!(missing, vec!["pass"]);
    }

    #[test]
    fn test_escaped_braces() {
        let fields = vec![("x", "1")];
        let (result, _) = expand_template("{{literal}} {x}", &fields);
        assert_eq!(result, "{literal} 1");
    }

    #[test]
    fn test_no_fields() {
        let (result, _) = expand_template("plain text", &[]);
        assert_eq!(result, "plain text");
    }
}
