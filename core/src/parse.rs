use crate::block::{BlockKind, Closing, Issue, Layout, ParsedBlock, Row};
use crate::defs::{collect_defs, find_def, BlockDef};
use crate::header::parse_header;
use crate::lexer::{classify_line, classify_line_with_defs, LineKind};
use crate::options::{LayoutMode, Separator};
use crate::patterns::{split_text_with_pairs, TextSlice};
use crate::schema::{Field, FieldType, Schema};

/// A segment of a note
#[derive(Debug, Clone)]
pub enum Segment {
    Text(TextSegment),
    Block(BlockSegment),
}

/// Free text segment
#[derive(Debug, Clone)]
pub struct TextSegment {
    pub text: String,
}

/// View state for a block
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewState {
    Table,
    Raw,
}

/// A block segment
#[derive(Debug, Clone)]
pub struct BlockSegment {
    /// The raw text of the block (including header and @fin)
    pub raw: String,
    /// Parsed representation
    pub parsed: ParsedBlock,
    /// Current view state (not persisted)
    pub state: ViewState,
}

/// End-of-line style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eol {
    Lf,
    CrLf,
}

impl Eol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
        }
    }
}

/// Detect the end-of-line style of the text
pub fn detect_eol(text: &str) -> Eol {
    if text.contains("\r\n") {
        Eol::CrLf
    } else {
        Eol::Lf
    }
}

/// Parse a full note text into segments
pub fn parse_note(text: &str) -> Vec<Segment> {
    // Normalize line endings for processing
    let normalized = text.replace("\r\n", "\n");
    let has_trailing_newline = normalized.ends_with('\n');
    let lines: Vec<&str> = normalized.split('\n').collect();

    let defs = collect_defs(&normalized);
    let def_names: Vec<String> = defs.iter().map(|d| d.name.to_lowercase()).collect();

    let mut classified: Vec<LineKind> = lines
        .iter()
        .map(|l| classify_line_with_defs(l, &def_names))
        .collect();

    // If the last line is an incomplete header line (no trailing newline, user still typing),
    // treat it as plain text until Enter is pressed.
    if !has_trailing_newline && !lines.is_empty() {
        let last_idx = lines.len() - 1;
        if matches!(classified[last_idx], LineKind::Header(_)) {
            classified[last_idx] = LineKind::Text;
        }
    }

    let mut segments = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        match &classified[i] {
            LineKind::Header(cmd) => {
                let (block_segment, end_idx) =
                    parse_block_from_lines(&lines, &classified, i, cmd, &defs);
                segments.push(Segment::Block(block_segment));
                i = end_idx;
            }
            _ => {
                // Accumulate text lines (includes @def, comments, free text)
                let start = i;
                while i < lines.len() {
                    match &classified[i] {
                        LineKind::Header(_) => break,
                        _ => i += 1,
                    }
                }
                let text_lines: Vec<&str> = lines[start..i].to_vec();
                let text = text_lines.join("\n");
                // Split free text into Text + auto-detected Pares blocks
                for slice in split_text_with_pairs(&text) {
                    match slice {
                        TextSlice::Plain(t) => {
                            segments.push(Segment::Text(TextSegment { text: t }));
                        }
                        TextSlice::Pares(parsed, raw) => {
                            segments.push(Segment::Block(BlockSegment {
                                raw,
                                parsed,
                                state: ViewState::Table,
                            }));
                        }
                    }
                }
            }
        }
    }

    segments
}

/// Parse a block starting at line index `start`
fn parse_block_from_lines(
    lines: &[&str],
    classified: &[LineKind],
    start: usize,
    cmd: &str,
    defs: &[BlockDef],
) -> (BlockSegment, usize) {
    let header_line = lines[start];

    // Extract the rest after the command name
    let rest = header_line
        .strip_prefix('@')
        .and_then(|s| {
            let cmd_end = s.find(|c: char| c.is_whitespace()).unwrap_or(s.len());
            if cmd_end < s.len() {
                Some(s[cmd_end..].trim_start())
            } else {
                Some("")
            }
        })
        .unwrap_or("");

    let parsed_header = parse_header(rest);

    // Resolve kind + schema/options (custom @def merges)
    let (kind, mut schema, options) = if let Some(builtin) = BlockKind::from_command(cmd) {
        (builtin, parsed_header.schema.clone(), parsed_header.options.clone())
    } else if let Some(def) = find_def(defs, cmd) {
        let merged_opts = def.options.merge_override(&parsed_header.options);
        let schema = if parsed_header.schema.fields.is_empty() {
            def.schema.clone()
        } else {
            parsed_header.schema.clone()
        };
        (BlockKind::Custom(cmd.to_string()), schema, merged_opts)
    } else {
        (BlockKind::Tabla, parsed_header.schema.clone(), parsed_header.options.clone())
    };

    // Default schema for @lista
    if matches!(kind, BlockKind::Lista) && schema.fields.is_empty() {
        schema = Schema::new(vec![
            Field::new("hecho", FieldType::Check),
            Field::new("item", FieldType::Text),
        ]);
    }

    // Find the end of the block body
    let mut body_lines: Vec<(usize, &str)> = Vec::new();
    let mut i = start + 1;
    let mut closing = Closing::EndOfFile;

    while i < lines.len() {
        match &classified[i] {
            LineKind::End => {
                closing = Closing::Fin;
                i += 1; // consume @fin
                break;
            }
            LineKind::Header(_) => {
                closing = Closing::NextBlock;
                break;
            }
            _ => {
                body_lines.push((i - start - 1, lines[i]));
                i += 1;
            }
        }
    }

    // Filter out comments and blank lines for data processing
    // (for lista keep blank-less; comments still filtered)
    let data_lines: Vec<(usize, &str)> = body_lines
        .iter()
        .filter(|(_, line)| {
            !matches!(classify_line(line), LineKind::Comment) && !line.trim().is_empty()
        })
        .cloned()
        .collect();

    let need_infer = schema.fields.is_empty();

    let layout = if matches!(kind, BlockKind::Lista) {
        Layout::Row
    } else {
        resolve_layout(&options.mode, &schema, &data_lines, &options.sep)
    };

    if need_infer && !data_lines.is_empty() && !matches!(kind, BlockKind::Lista) {
        let first_line = data_lines[0].1;
        let token_count = tokenize_line(first_line, &options.sep).len();
        schema = Schema::inferred(token_count.max(1));
    }

    let (rows, issues) = if matches!(kind, BlockKind::Lista) {
        parse_lista_body(&data_lines, &schema)
    } else {
        parse_body(&data_lines, &schema, &layout, &options.sep)
    };

    let raw_lines: Vec<&str> = lines[start..i].to_vec();
    let raw = raw_lines.join("\n");

    let view_state = match options.view {
        crate::options::ViewDefault::Table => ViewState::Table,
        crate::options::ViewDefault::Raw => ViewState::Raw,
    };

    let parsed = ParsedBlock {
        kind,
        title: parsed_header.title,
        schema,
        options,
        layout,
        rows,
        issues,
        closed: closing,
    };

    let segment = BlockSegment {
        raw,
        parsed,
        state: view_state,
    };

    (segment, i)
}

/// Resolve the layout mode
fn resolve_layout(
    mode: &LayoutMode,
    schema: &Schema,
    data_lines: &[(usize, &str)],
    sep: &Separator,
) -> Layout {
    match mode {
        LayoutMode::Row => Layout::Row,
        LayoutMode::Vertical => Layout::Vertical,
        LayoutMode::Auto => {
            let n = schema.field_count();
            if n <= 1 {
                return Layout::Row;
            }
            if data_lines.is_empty() {
                return Layout::Row;
            }
            let single_token_count = data_lines
                .iter()
                .filter(|(_, line)| tokenize_line(line, sep).len() == 1)
                .count();
            if single_token_count > data_lines.len() / 2 {
                Layout::Vertical
            } else {
                Layout::Row
            }
        }
    }
}

/// Tokenize a line based on the separator
pub fn tokenize_line(line: &str, sep: &Separator) -> Vec<String> {
    match sep {
        Separator::Space => tokenize_space(line),
        _ => {
            let sep_char = sep.char().unwrap();
            line.split(sep_char).map(|s| s.trim().to_string()).collect()
        }
    }
}

/// Tokenize by spaces, supporting quoted strings
fn tokenize_space(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' if !in_quotes => {
                in_quotes = true;
            }
            '"' if in_quotes => {
                in_quotes = false;
                tokens.push(current.clone());
                current.clear();
            }
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

/// Parse @lista body: `[ ] item` / `[x] item` or plain lines
fn parse_lista_body(data_lines: &[(usize, &str)], schema: &Schema) -> (Vec<Row>, Vec<Issue>) {
    let mut rows = Vec::new();
    let issues = Vec::new();
    let n = schema.field_count().max(2);

    for (line_idx, line) in data_lines {
        let trimmed = line.trim();
        let (check, item) = if let Some(rest) = trimmed.strip_prefix("[x]")
            .or_else(|| trimmed.strip_prefix("[X]"))
            .or_else(|| trimmed.strip_prefix("[✓]"))
        {
            ("[x]".to_string(), rest.trim().to_string())
        } else if let Some(rest) = trimmed.strip_prefix("[ ]")
            .or_else(|| trimmed.strip_prefix("[]"))
        {
            ("[ ]".to_string(), rest.trim().to_string())
        } else {
            ("[ ]".to_string(), trimmed.to_string())
        };

        let mut cells = vec![check, item];
        while cells.len() < n {
            cells.push(String::new());
        }
        cells.truncate(n);

        rows.push(Row {
            cells,
            src_lines: *line_idx..*line_idx + 1,
        });
    }

    (rows, issues)
}

/// Parse the body lines into rows
fn parse_body(
    data_lines: &[(usize, &str)],
    schema: &Schema,
    layout: &Layout,
    sep: &Separator,
) -> (Vec<Row>, Vec<Issue>) {
    let n = schema.field_count();
    if n == 0 || data_lines.is_empty() {
        return (Vec::new(), Vec::new());
    }

    match layout {
        Layout::Row => parse_body_row(data_lines, n, sep),
        Layout::Vertical => parse_body_vertical(data_lines, n),
        Layout::KeyValue => parse_body_vertical(data_lines, n),
    }
}

fn parse_body_row(
    data_lines: &[(usize, &str)],
    field_count: usize,
    sep: &Separator,
) -> (Vec<Row>, Vec<Issue>) {
    let mut rows = Vec::new();
    let mut issues = Vec::new();

    for (line_idx, line) in data_lines {
        let mut tokens = tokenize_line(line, sep);

        if tokens.len() > field_count {
            let excess: Vec<String> = tokens.drain(field_count - 1..).collect();
            let sep_str = match sep {
                Separator::Space => " ",
                Separator::Tab => "\t",
                Separator::Pipe => "|",
                Separator::Comma => ",",
                Separator::Semicolon => ";",
            };
            tokens.push(excess.join(sep_str));
        }

        if tokens.len() < field_count {
            issues.push(Issue {
                line: *line_idx,
                message: format!(
                    "Expected {} fields, found {}",
                    field_count,
                    tokens.len()
                ),
            });
            while tokens.len() < field_count {
                tokens.push(String::new());
            }
        }

        rows.push(Row {
            cells: tokens,
            src_lines: *line_idx..*line_idx + 1,
        });
    }

    (rows, issues)
}

fn parse_body_vertical(
    data_lines: &[(usize, &str)],
    field_count: usize,
) -> (Vec<Row>, Vec<Issue>) {
    let mut rows = Vec::new();
    let mut issues = Vec::new();

    let chunks: Vec<Vec<(usize, &str)>> = data_lines
        .chunks(field_count)
        .map(|chunk| chunk.to_vec())
        .collect();

    for chunk in chunks {
        let start_line = chunk[0].0;
        let end_line = chunk.last().map(|(idx, _)| idx + 1).unwrap_or(start_line + 1);

        let mut cells: Vec<String> = chunk.iter().map(|(_, line)| line.to_string()).collect();

        if cells.len() < field_count {
            issues.push(Issue {
                line: start_line,
                message: format!(
                    "Incomplete record: expected {} fields, found {}",
                    field_count,
                    cells.len()
                ),
            });
            while cells.len() < field_count {
                cells.push(String::new());
            }
        }

        rows.push(Row {
            cells,
            src_lines: start_line..end_line,
        });
    }

    (rows, issues)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_block() {
        let text = "@tabla \"Correos\" correo:email, vence:fecha\ntest@gmail.com 12/11\nana@gmail.com 03/12\n@fin";
        let segments = parse_note(text);
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            Segment::Block(b) => {
                assert_eq!(b.parsed.title, Some("Correos".to_string()));
                assert_eq!(b.parsed.rows.len(), 2);
                assert_eq!(b.parsed.schema.fields.len(), 2);
                assert_eq!(b.parsed.closed, Closing::Fin);
            }
            _ => panic!("Expected block"),
        }
    }

    #[test]
    fn test_parse_text_and_block() {
        let text = "Some intro text\n\n@tabla ip, user\n192.168.1.1 admin\n@fin\n\nMore text";
        let segments = parse_note(text);
        assert!(segments.len() >= 3);
        assert!(matches!(segments[0], Segment::Text(_)));
        assert!(segments.iter().any(|s| matches!(s, Segment::Block(_))));
    }

    #[test]
    fn test_block_without_fin() {
        let text = "@tabla ip, user\n192.168.1.1 admin";
        let segments = parse_note(text);
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            Segment::Block(b) => {
                assert_eq!(b.parsed.closed, Closing::EndOfFile);
                assert_eq!(b.parsed.rows.len(), 1);
            }
            _ => panic!("Expected block"),
        }
    }

    #[test]
    fn test_vertical_layout() {
        let text = "@registro \"Router\" ip, user, pass | modo=vertical\n192.168.0.1\nadmin\nrouter2026\n@fin";
        let segments = parse_note(text);
        match &segments[0] {
            Segment::Block(b) => {
                assert_eq!(b.parsed.layout, Layout::Vertical);
                assert_eq!(b.parsed.rows.len(), 1);
                assert_eq!(b.parsed.rows[0].cells[0], "192.168.0.1");
                assert_eq!(b.parsed.rows[0].cells[1], "admin");
                assert_eq!(b.parsed.rows[0].cells[2], "router2026");
            }
            _ => panic!("Expected block"),
        }
    }

    #[test]
    fn test_escaped_at() {
        let text = "\\@tabla no abre bloque";
        let segments = parse_note(text);
        assert_eq!(segments.len(), 1);
        assert!(matches!(segments[0], Segment::Text(_)));
    }

    #[test]
    fn test_comment_in_block() {
        let text = "@tabla ip, user\n@// ignore this\n192.168.1.1 admin\n@fin";
        let segments = parse_note(text);
        match &segments[0] {
            Segment::Block(b) => {
                assert_eq!(b.parsed.rows.len(), 1);
            }
            _ => panic!("Expected block"),
        }
    }

    #[test]
    fn test_tokenize_with_quotes() {
        let tokens = tokenize_space("\"Juan Pérez\" juan@mail.com");
        assert_eq!(tokens, vec!["Juan Pérez", "juan@mail.com"]);
    }

    #[test]
    fn test_last_field_absorbs() {
        let text = "@tabla name, description\nAlice This is a long description\n@fin";
        let segments = parse_note(text);
        match &segments[0] {
            Segment::Block(b) => {
                assert_eq!(b.parsed.rows[0].cells[0], "Alice");
                assert_eq!(b.parsed.rows[0].cells[1], "This is a long description");
            }
            _ => panic!("Expected block"),
        }
    }

    #[test]
    fn test_schema_inference() {
        let text = "@tabla\nfoo bar baz\none two three\n@fin";
        let segments = parse_note(text);
        match &segments[0] {
            Segment::Block(b) => {
                assert_eq!(b.parsed.schema.fields.len(), 3);
                assert_eq!(b.parsed.schema.fields[0].name, "col1");
                assert_eq!(b.parsed.rows.len(), 2);
            }
            _ => panic!("Expected block"),
        }
    }

    #[test]
    fn test_incomplete_header_remains_text() {
        let text = "@tabla";
        let segments = parse_note(text);
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            Segment::Text(t) => assert_eq!(t.text, "@tabla"),
            _ => panic!("Expected text segment for incomplete header"),
        }

        let text2 = "@tabla id, nombre, codigo";
        let segments2 = parse_note(text2);
        assert_eq!(segments2.len(), 1);
        match &segments2[0] {
            Segment::Text(t) => assert_eq!(t.text, "@tabla id, nombre, codigo"),
            _ => panic!("Expected text segment while typing header without Enter"),
        }
    }

    #[test]
    fn test_complete_header_with_newline_becomes_block() {
        let text = "@tabla id, nombre, codigo\n";
        let segments = parse_note(text);
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            Segment::Block(b) => {
                assert_eq!(b.parsed.schema.fields.len(), 3);
                assert_eq!(b.parsed.schema.fields[0].name, "id");
                assert_eq!(b.parsed.schema.fields[1].name, "nombre");
                assert_eq!(b.parsed.schema.fields[2].name, "codigo");
            }
            _ => panic!("Expected block segment once newline is present"),
        }
    }

    #[test]
    fn test_lista() {
        let text = "@lista \"Tareas\"\n[ ] Comprar pan\n[x] Enviar mail\n@fin";
        let segments = parse_note(text);
        match &segments[0] {
            Segment::Block(b) => {
                assert_eq!(b.parsed.kind, BlockKind::Lista);
                assert_eq!(b.parsed.rows.len(), 2);
                assert_eq!(b.parsed.rows[0].cells[0], "[ ]");
                assert_eq!(b.parsed.rows[0].cells[1], "Comprar pan");
                assert_eq!(b.parsed.rows[1].cells[0], "[x]");
                assert_eq!(b.parsed.rows[1].cells[1], "Enviar mail");
            }
            _ => panic!("Expected lista block"),
        }
    }

    #[test]
    fn test_def_custom_block() {
        let text = "\
@def ftp ip, user, pass:secreto | copiar=ftp://{user}:{pass}@{ip}

@ftp \"Oficina\"
192.168.1.10 admin 1234
@fin
";
        let segments = parse_note(text);
        let block = segments.iter().find_map(|s| match s {
            Segment::Block(b) => Some(b),
            _ => None,
        }).expect("custom block");
        assert!(matches!(block.parsed.kind, BlockKind::Custom(ref n) if n == "ftp"));
        assert_eq!(block.parsed.schema.fields.len(), 3);
        assert_eq!(block.parsed.rows.len(), 1);
        assert_eq!(block.parsed.rows[0].cells[0], "192.168.1.10");
        assert!(block.parsed.options.copy_template.is_some());
    }

    #[test]
    fn test_auto_detect_pares() {
        let text = "\
FTP PSICOLOGIA
servidor FTP    	: ftp.example.com
usuario de FTP 	: user@example.com
contraseña      	: secret
directorio      	: /home/path
";
        let segments = parse_note(text);
        let block = segments.iter().find_map(|s| match s {
            Segment::Block(b) if b.parsed.kind == BlockKind::Pares => Some(b),
            _ => None,
        }).expect("pares block");
        assert_eq!(block.parsed.title.as_deref(), Some("FTP PSICOLOGIA"));
        assert_eq!(block.parsed.schema.fields.len(), 4);
    }
}
