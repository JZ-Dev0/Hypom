use std::path::{Path, PathBuf};
use hypom_core::parse::parse_note;
use hypom_core::schema::FieldType;
use crate::workspace::Workspace;

#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub note_path: PathBuf,
    pub note_title: String,
    pub project_name: Option<String>,
    pub block_title: Option<String>,
    pub text: String,
    pub is_row: bool,
    pub row_cells: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SearchIndex {
    pub entries: Vec<IndexEntry>,
}

impl SearchIndex {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn build_from_workspace(&mut self, workspace: &Workspace) {
        self.entries.clear();

        // Index general notes
        for note_ref in &workspace.general {
            self.index_note_file(&note_ref.path, &note_ref.title, None);
        }

        // Index project notes
        for project in &workspace.projects {
            let proj_name = project.display_name().to_string();
            for note_ref in &project.notes {
                self.index_note_file(&note_ref.path, &note_ref.title, Some(&proj_name));
            }
        }
    }

    pub fn index_note_file(&mut self, path: &Path, title: &str, project_name: Option<&str>) {
        // Entry for the note itself
        self.entries.push(IndexEntry {
            note_path: path.to_path_buf(),
            note_title: title.to_string(),
            project_name: project_name.map(|s| s.to_string()),
            block_title: None,
            text: format!("{} {}", title, project_name.unwrap_or("General")),
            is_row: false,
            row_cells: Vec::new(),
        });

        // Read and index content
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) => return,
        };

        let segments = parse_note(&text);
        for seg in segments {
            if let hypom_core::parse::Segment::Block(b) = seg {
                let block_title = b.parsed.title.clone();
                let schema = &b.parsed.schema;

                for row in &b.parsed.rows {
                    // Collect only non-secret cells
                    let mut visible_cells = Vec::new();
                    for (i, cell) in row.cells.iter().enumerate() {
                        let is_secret = schema
                            .fields
                            .get(i)
                            .map(|f| f.ty == FieldType::Secret)
                            .unwrap_or(false);

                        if !is_secret && !cell.trim().is_empty() {
                            visible_cells.push(cell.clone());
                        }
                    }

                    if !visible_cells.is_empty() {
                        let combined_text = visible_cells.join(" ");
                        self.entries.push(IndexEntry {
                            note_path: path.to_path_buf(),
                            note_title: title.to_string(),
                            project_name: project_name.map(|s| s.to_string()),
                            block_title: block_title.clone(),
                            text: combined_text,
                            is_row: true,
                            row_cells: row.cells.clone(),
                        });
                    }
                }
            }
        }
    }
}

