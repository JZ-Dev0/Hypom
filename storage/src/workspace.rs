use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Workspace {
    pub root: PathBuf,
    pub general: Vec<NoteRef>,
    pub projects: Vec<Project>,
}

#[derive(Debug, Clone)]
pub struct Project {
    pub name: String,
    pub path: PathBuf,
    pub meta: ProjectMeta,
    pub notes: Vec<NoteRef>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub nombre: Option<String>,
    pub color: Option<String>,
    pub icono: Option<String>,
    pub orden: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct NoteRef {
    pub path: PathBuf,
    pub title: String,
    pub modified: SystemTime,
}

impl NoteRef {
    pub fn from_path(path: PathBuf) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_lowercase();
        if ext != "txt" && ext != "md" {
            return None;
        }

        let title = path.file_stem()?.to_string_lossy().to_string();
        let modified = fs::metadata(&path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        Some(Self {
            path,
            title,
            modified,
        })
    }
}

pub fn scan_workspace(root: &Path) -> Result<Workspace, std::io::Error> {
    if !root.exists() {
        fs::create_dir_all(root)?;
    }

    let mut general_notes = Vec::new();
    let mut projects = Vec::new();

    let general_dir = root.join("general");
    if general_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&general_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    if let Some(note_ref) = NoteRef::from_path(p) {
                        general_notes.push(note_ref);
                    }
                }
            }
        }
    }

    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let p = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            if file_name.starts_with('.') {
                continue;
            }

            if p.is_file() {
                if let Some(note_ref) = NoteRef::from_path(p) {
                    general_notes.push(note_ref);
                }
            } else if p.is_dir() && file_name.to_lowercase() != "general" {
                let mut project_meta = ProjectMeta::default();
                let meta_path = p.join("proyecto.toml");
                if meta_path.is_file() {
                    if let Ok(content) = fs::read_to_string(&meta_path) {
                        if let Ok(meta) = toml::from_str::<ProjectMeta>(&content) {
                            project_meta = meta;
                        }
                    }
                }

                let mut project_notes = Vec::new();
                if let Ok(sub_entries) = fs::read_dir(&p) {
                    for sub in sub_entries.flatten() {
                        let sub_p = sub.path();
                        if sub_p.is_file() {
                            if let Some(note_ref) = NoteRef::from_path(sub_p) {
                                project_notes.push(note_ref);
                            }
                        }
                    }
                }

                project_notes.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));

                projects.push(Project {
                    name: file_name,
                    path: p,
                    meta: project_meta,
                    notes: project_notes,
                });
            }
        }
    }

    general_notes.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    projects.sort_by(|a, b| {
        let order_a = a.meta.orden.unwrap_or(i32::MAX);
        let order_b = b.meta.orden.unwrap_or(i32::MAX);
        order_a.cmp(&order_b).then_with(|| a.display_name().to_lowercase().cmp(&b.display_name().to_lowercase()))
    });

    Ok(Workspace {
        root: root.to_path_buf(),
        general: general_notes,
        projects,
    })
}

impl Project {
    pub fn display_name(&self) -> &str {
        self.meta.nombre.as_deref().unwrap_or(&self.name)
    }
}

pub fn create_note(
    root: &Path,
    project_folder: Option<&str>,
    title: &str,
) -> Result<PathBuf, std::io::Error> {
    let clean_title = if title.ends_with(".txt") || title.ends_with(".md") {
        title.to_string()
    } else {
        format!("{}.txt", title)
    };

    let target_dir = match project_folder {
        Some(proj) if !proj.is_empty() => root.join(proj),
        _ => {
            let gen = root.join("general");
            if !gen.exists() {
                fs::create_dir_all(&gen)?;
            }
            gen
        }
    };

    if !target_dir.exists() {
        fs::create_dir_all(&target_dir)?;
    }

    let note_path = target_dir.join(&clean_title);
    if !note_path.exists() {
        fs::write(&note_path, "")?;
    }

    Ok(note_path)
}

pub fn create_project(root: &Path, name: &str) -> Result<PathBuf, std::io::Error> {
    let proj_dir = root.join(name);
    if !proj_dir.exists() {
        fs::create_dir_all(&proj_dir)?;
    }
    Ok(proj_dir)
}

