use crate::block::{ParsedBlock, Row};
use crate::dates::{closest_sort_key, parse_date, DateOrder};
use crate::options::SortOrder;
use crate::schema::FieldType;
use crate::template::expand_template;

/// Sort rows by a column
pub fn sort_rows(block: &mut ParsedBlock, date_order: &DateOrder) {
    let order = match &block.options.order {
        Some(o) => o.clone(),
        None => return,
    };

    let (field_name, is_desc, is_closest) = match &order {
        SortOrder::Asc(f) => (f.clone(), false, false),
        SortOrder::Desc(f) => (f.clone(), true, false),
        SortOrder::Closest(f) => (f.clone(), false, true),
    };

    let field_idx = match block.schema.field_index(&field_name) {
        Some(idx) => idx,
        None => return,
    };

    let field_type = &block.schema.fields[field_idx].ty;

    block.rows.sort_by(|a, b| {
        let va = a.cells.get(field_idx).map(|s| s.as_str()).unwrap_or("");
        let vb = b.cells.get(field_idx).map(|s| s.as_str()).unwrap_or("");

        let cmp = match field_type {
            FieldType::Number => {
                let na: f64 = va.replace(',', ".").parse().unwrap_or(f64::MAX);
                let nb: f64 = vb.replace(',', ".").parse().unwrap_or(f64::MAX);
                na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal)
            }
            FieldType::Date if is_closest => {
                let da = parse_date(va, date_order);
                let db = parse_date(vb, date_order);
                match (da, db) {
                    (Some(a), Some(b)) => closest_sort_key(a).cmp(&closest_sort_key(b)),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            }
            FieldType::Date => {
                let da = parse_date(va, date_order);
                let db = parse_date(vb, date_order);
                match (da, db) {
                    (Some(a), Some(b)) => a.cmp(&b),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            }
            _ => {
                // Text comparison, case-insensitive
                va.to_lowercase().cmp(&vb.to_lowercase())
            }
        };

        if is_desc { cmp.reverse() } else { cmp }
    });
}

/// Edit a single cell value
pub fn edit_cell(block: &mut ParsedBlock, row_idx: usize, col_idx: usize, new_value: String) {
    if let Some(row) = block.rows.get_mut(row_idx) {
        if col_idx < row.cells.len() {
            row.cells[col_idx] = new_value;
        }
    }
}

/// Add a new empty row
pub fn add_row(block: &mut ParsedBlock) {
    let n = block.schema.field_count();
    let src_start = block.rows.last().map(|r| r.src_lines.end).unwrap_or(0);
    block.rows.push(Row {
        cells: vec![String::new(); n],
        src_lines: src_start..src_start + 1,
    });
}

/// Remove a row by index
pub fn remove_row(block: &mut ParsedBlock, row_idx: usize) -> Option<Row> {
    if row_idx < block.rows.len() {
        Some(block.rows.remove(row_idx))
    } else {
        None
    }
}

/// Copy a cell value
pub fn copy_cell(block: &ParsedBlock, row_idx: usize, col_idx: usize) -> Option<String> {
    block.rows.get(row_idx)?.cells.get(col_idx).cloned()
}

/// Copy a full row using the copy template or tab-separated values
pub fn copy_row(block: &ParsedBlock, row_idx: usize) -> Option<String> {
    let row = block.rows.get(row_idx)?;

    if let Some(template) = &block.options.copy_template {
        let fields: Vec<(&str, &str)> = block
            .schema
            .fields
            .iter()
            .zip(row.cells.iter())
            .map(|(f, v)| (f.name.as_str(), v.as_str()))
            .collect();
        let (result, _) = expand_template(template, &fields);
        Some(result)
    } else {
        Some(row.cells.join("\t"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_note;
    use crate::parse::Segment;

    fn get_block(text: &str) -> ParsedBlock {
        let segments = parse_note(text);
        match segments.into_iter().next().unwrap() {
            Segment::Block(b) => b.parsed,
            _ => panic!("Expected block"),
        }
    }

    #[test]
    fn test_sort_text_asc() {
        let mut block = get_block("@tabla name\nCharlie\nAlice\nBob\n@fin");
        block.options.order = Some(SortOrder::Asc("name".to_string()));
        sort_rows(&mut block, &DateOrder::Dmy);
        let names: Vec<&str> = block.rows.iter().map(|r| r.cells[0].as_str()).collect();
        assert_eq!(names, vec!["Alice", "Bob", "Charlie"]);
    }

    #[test]
    fn test_sort_text_desc() {
        let mut block = get_block("@tabla name\nAlice\nBob\nCharlie\n@fin");
        block.options.order = Some(SortOrder::Desc("name".to_string()));
        sort_rows(&mut block, &DateOrder::Dmy);
        let names: Vec<&str> = block.rows.iter().map(|r| r.cells[0].as_str()).collect();
        assert_eq!(names, vec!["Charlie", "Bob", "Alice"]);
    }

    #[test]
    fn test_edit_cell() {
        let mut block = get_block("@tabla ip, user\n192.168.1.1 admin\n@fin");
        edit_cell(&mut block, 0, 1, "root".to_string());
        assert_eq!(block.rows[0].cells[1], "root");
    }

    #[test]
    fn test_add_and_remove_row() {
        let mut block = get_block("@tabla ip, user\n192.168.1.1 admin\n@fin");
        assert_eq!(block.rows.len(), 1);
        add_row(&mut block);
        assert_eq!(block.rows.len(), 2);
        assert_eq!(block.rows[1].cells, vec!["", ""]);
        remove_row(&mut block, 1);
        assert_eq!(block.rows.len(), 1);
    }

    #[test]
    fn test_copy_cell() {
        let block = get_block("@tabla ip, user\n192.168.1.1 admin\n@fin");
        assert_eq!(copy_cell(&block, 0, 0), Some("192.168.1.1".to_string()));
        assert_eq!(copy_cell(&block, 0, 1), Some("admin".to_string()));
        assert_eq!(copy_cell(&block, 5, 0), None);
    }

    #[test]
    fn test_copy_row_no_template() {
        let block = get_block("@tabla ip, user\n192.168.1.1 admin\n@fin");
        let result = copy_row(&block, 0).unwrap();
        assert_eq!(result, "192.168.1.1\tadmin");
    }

    #[test]
    fn test_copy_row_with_template() {
        let block = get_block(
            "@tabla ip, user, pass | copiar=ftp://{user}:{pass}@{ip}\n192.168.1.1 admin secret123\n@fin",
        );
        let result = copy_row(&block, 0).unwrap();
        assert_eq!(result, "ftp://admin:secret123@192.168.1.1");
    }
}
