use crate::block::{ParsedBlock, Row};
use crate::dates::{closest_sort_key, days_from_today, parse_date, DateOrder};
use crate::options::{AlertOpt, FilterOpt, SortOrder};
use crate::schema::FieldType;
use crate::template::expand_template;
use std::collections::HashSet;

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

/// Returns row indices that pass the view-only filter (or all if no filter).
pub fn visible_row_indices(block: &ParsedBlock) -> Vec<usize> {
    match &block.options.filter {
        None => (0..block.rows.len()).collect(),
        Some(f) => filtered_indices(block, f),
    }
}

fn filtered_indices(block: &ParsedBlock, filter: &FilterOpt) -> Vec<usize> {
    let Some(idx) = block.schema.field_index(&filter.field) else {
        return (0..block.rows.len()).collect();
    };
    let needle = filter.text.to_lowercase();
    block
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| {
            row.cells
                .get(idx)
                .map(|c| c.to_lowercase().contains(&needle))
                .unwrap_or(false)
        })
        .map(|(i, _)| i)
        .collect()
}

/// Sum a numeric column (respecting current filter if any).
pub fn column_total(block: &ParsedBlock) -> Option<(String, f64)> {
    let field_name = block.options.total.as_ref()?;
    let idx = block.schema.field_index(field_name)?;
    if block.schema.fields[idx].ty != FieldType::Number
        && block.schema.fields[idx].ty != FieldType::Text
    {
        // Still try to parse numbers even if typed as text
    }
    let mut sum = 0.0;
    for row_idx in visible_row_indices(block) {
        let val = block.rows[row_idx]
            .cells
            .get(idx)
            .map(|s| s.replace(',', "."))
            .unwrap_or_default();
        if let Ok(n) = val.trim().parse::<f64>() {
            sum += n;
        }
    }
    Some((field_name.clone(), sum))
}

/// Returns true if the cell at (row, col) should be alert-highlighted.
pub fn is_alert_cell(block: &ParsedBlock, row_idx: usize, col_idx: usize, date_order: &DateOrder) -> bool {
    let Some(alert) = &block.options.alert else {
        return false;
    };
    is_date_in_alert_window(block, row_idx, col_idx, alert, date_order)
}

fn is_date_in_alert_window(
    block: &ParsedBlock,
    row_idx: usize,
    col_idx: usize,
    alert: &AlertOpt,
    date_order: &DateOrder,
) -> bool {
    let Some(field_idx) = block.schema.field_index(&alert.field) else {
        return false;
    };
    if field_idx != col_idx {
        return false;
    }
    let Some(cell) = block.rows.get(row_idx).and_then(|r| r.cells.get(col_idx)) else {
        return false;
    };
    let Some(date) = parse_date(cell, date_order) else {
        return false;
    };
    let days = days_from_today(date);
    days >= 0 && days <= alert.days
}

/// Row indices whose value in `unico` field is duplicated.
pub fn duplicate_row_indices(block: &ParsedBlock) -> HashSet<usize> {
    let Some(field) = &block.options.unique else {
        return HashSet::new();
    };
    let Some(idx) = block.schema.field_index(field) else {
        return HashSet::new();
    };

    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut dups = HashSet::new();

    for (row_idx, row) in block.rows.iter().enumerate() {
        let val = row
            .cells
            .get(idx)
            .map(|s| s.trim().to_lowercase())
            .unwrap_or_default();
        if val.is_empty() {
            continue;
        }
        if let Some(first) = seen.get(&val) {
            dups.insert(*first);
            dups.insert(row_idx);
        } else {
            seen.insert(val, row_idx);
        }
    }
    dups
}

/// Parse check cell display / normalize
pub fn is_checked(value: &str) -> bool {
    let v = value.trim().to_lowercase();
    matches!(
        v.as_str(),
        "[x]" | "x" | "si" | "sí" | "true" | "1" | "yes" | "✓"
    )
}

pub fn toggle_check(value: &str) -> String {
    if is_checked(value) {
        "[ ]".to_string()
    } else {
        "[x]".to_string()
    }
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
    let cells = if matches!(block.kind, crate::block::BlockKind::Lista) && n >= 1 {
        let mut c = vec![String::new(); n];
        c[0] = "[ ]".to_string();
        c
    } else {
        vec![String::new(); n]
    };
    block.rows.push(Row {
        cells,
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

    #[test]
    fn test_filter_and_total() {
        let block = get_block(
            "@tabla item, monto:numero | filtro=item~av | total=monto\napple 10\nbanana 20\navocado 5\n@fin",
        );
        let visible = visible_row_indices(&block);
        assert_eq!(visible.len(), 1); // avocado
        let (name, sum) = column_total(&block).unwrap();
        assert_eq!(name, "monto");
        assert!((sum - 5.0).abs() < f64::EPSILON);
    }
}
