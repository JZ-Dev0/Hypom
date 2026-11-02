/// Classification of a line for the parser
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineKind {
    /// Regular text (not a command)
    Text,
    /// Comment: `@//`
    Comment,
    /// Block header: `@tabla`, `@registro`, or custom
    Header(String),  // the command name
    /// End of block: `@fin`
    End,
    /// Definition: `@def`
    Def,
    /// Escaped line: starts with `\@`
    Escaped,
}

/// Known block commands (v1)
pub const BLOCK_COMMANDS: &[&str] = &["tabla", "registro"];

/// Known line commands (v1)
pub const LINE_COMMANDS: &[&str] = &["fin", "//", "def"];

/// All known commands
pub fn is_known_command(name: &str) -> bool {
    let lower = name.to_lowercase();
    BLOCK_COMMANDS.iter().any(|c| *c == lower)
        || LINE_COMMANDS.iter().any(|c| *c == lower)
}

/// Returns true if the command name is a block-opening command
pub fn is_block_command(name: &str) -> bool {
    let lower = name.to_lowercase();
    BLOCK_COMMANDS.iter().any(|c| *c == lower)
}

/// Classify a single line
pub fn classify_line(line: &str) -> LineKind {
    // Must start with @ at position 0
    if !line.starts_with('@') {
        // Check for escaped @
        if line.starts_with("\\@") {
            return LineKind::Escaped;
        }
        return LineKind::Text;
    }

    // Get the command name (everything after @ until whitespace or end)
    let rest = &line[1..];

    // Special case: @// comment
    if rest.starts_with("//") {
        return LineKind::Comment;
    }

    // Extract command name
    let cmd_end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
    let cmd = &rest[..cmd_end];

    if cmd.is_empty() {
        return LineKind::Text;
    }

    let cmd_lower = cmd.to_lowercase();

    match cmd_lower.as_str() {
        "fin" => LineKind::End,
        "def" => LineKind::Def,
        _ if BLOCK_COMMANDS.iter().any(|c| *c == cmd_lower) => {
            LineKind::Header(cmd.to_string())
        }
        _ => {
            // Unknown @ command — treat as text (not an error)
            LineKind::Text
        }
    }
}

/// Classify all lines in a document, returning (line_content, kind) pairs
pub fn classify_lines(text: &str) -> Vec<(String, LineKind)> {
    text.lines()
        .map(|line| (line.to_string(), classify_line(line)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_line() {
        assert_eq!(classify_line("hello world"), LineKind::Text);
    }

    #[test]
    fn test_at_in_middle() {
        assert_eq!(classify_line("test@gmail.com"), LineKind::Text);
    }

    #[test]
    fn test_comment() {
        assert_eq!(classify_line("@// this is a comment"), LineKind::Comment);
    }

    #[test]
    fn test_header_tabla() {
        assert_eq!(
            classify_line("@tabla \"Correos\" correo:email"),
            LineKind::Header("tabla".to_string())
        );
    }

    #[test]
    fn test_header_registro() {
        assert_eq!(
            classify_line("@registro \"FTP\""),
            LineKind::Header("registro".to_string())
        );
    }

    #[test]
    fn test_end() {
        assert_eq!(classify_line("@fin"), LineKind::End);
    }

    #[test]
    fn test_def() {
        assert_eq!(classify_line("@def ftp ip, user, pass"), LineKind::Def);
    }

    #[test]
    fn test_escaped() {
        assert_eq!(classify_line("\\@tabla no abre bloque"), LineKind::Escaped);
    }

    #[test]
    fn test_unknown_command_is_text() {
        assert_eq!(classify_line("@unknown stuff"), LineKind::Text);
    }

    #[test]
    fn test_bare_at() {
        assert_eq!(classify_line("@"), LineKind::Text);
    }

    #[test]
    fn test_at_mention() {
        assert_eq!(classify_line("@juan"), LineKind::Text);
    }
}
