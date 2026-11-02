use crate::block::Row;
use crate::options::Separator;
use crate::parse::Segment;

/// Serialize a single row back to text (row mode)
pub fn serialize_row(row: &Row, sep: &Separator) -> String {
    let sep_str = match sep {
        Separator::Space => " ",
        Separator::Tab => "\t",
        Separator::Pipe => " | ",
        Separator::Comma => ", ",
        Separator::Semicolon => "; ",
    };

    // Check if any non-last cell needs quoting (contains the separator character)
    let cells: Vec<String> = row
        .cells
        .iter()
        .enumerate()
        .map(|(i, cell)| {
            if matches!(sep, Separator::Space) {
                if cell.is_empty() {
                    "\"\"".to_string()
                } else if cell.contains(' ') && i < row.cells.len() - 1 {
                    format!("\"{}\"", cell)
                } else {
                    cell.clone()
                }
            } else {
                cell.clone()
            }
        })
        .collect();

    cells.join(sep_str)
}

/// Serialize a row in vertical mode
pub fn serialize_row_vertical(row: &Row) -> String {
    row.cells.join("\n")
}

/// Serialize all segments back to text
pub fn serialize_segments(segments: &[Segment], eol: &str) -> String {
    segments
        .iter()
        .map(|seg| match seg {
            Segment::Text(t) => t.text.clone(),
            Segment::Block(b) => b.raw.clone(),
        })
        .collect::<Vec<_>>()
        .join(eol)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_note;

    #[test]
    fn test_roundtrip_simple() {
        let text = "Hello\n\n@tabla ip, user\n192.168.1.1 admin\n10.0.0.5 root\n@fin\n\nGoodbye";
        let segments = parse_note(text);
        let result = serialize_segments(&segments, "\n");
        assert_eq!(result, text);
    }

    #[test]
    fn test_roundtrip_vertical() {
        let text = "@registro \"R\" ip, user, pass | modo=vertical\n192.168.0.1\nadmin\nrouter2026\n@fin";
        let segments = parse_note(text);
        let result = serialize_segments(&segments, "\n");
        assert_eq!(result, text);
    }

    #[test]
    fn test_roundtrip_comment() {
        let text = "@tabla ip\n@// a comment\n192.168.1.1\n@fin";
        let segments = parse_note(text);
        let result = serialize_segments(&segments, "\n");
        assert_eq!(result, text);
    }
}
