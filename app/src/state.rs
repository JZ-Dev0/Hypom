use std::path::{Path, PathBuf};
use std::time::Instant;
use hypom_core::dates::DateOrder;
use hypom_storage::{
    create_note, create_project, load_note, save_note, scan_workspace,
    AutosaveManager, Config, LoadedNote, SearchIndex, Workspace, WorkspaceEvent, WorkspaceWatcher,
};
use crate::clipboard::ClipboardManager;
use crate::editor::EditorUiState;
use crate::palette::PaletteState;

pub struct AppState {
    pub config: Config,
    pub workspace: Option<Workspace>,
    pub watcher: Option<WorkspaceWatcher>,
    pub open_notes: Vec<LoadedNote>,
    pub active_note_idx: Option<usize>,
    pub autosave: AutosaveManager,
    pub clipboard: ClipboardManager,
    pub search_index: SearchIndex,
    pub palette_open: bool,
    pub palette_state: PaletteState,
    pub editor_state: EditorUiState,
    pub sidebar_visible: bool,
    pub toast: Option<(String, Instant)>,
    pub date_order: DateOrder,
    pub new_note_dialog: Option<Option<String>>, // target project folder
    pub new_note_input: String,
    pub new_project_dialog: bool,
    pub new_project_input: String,
    pub dialog_just_opened: bool,
}

impl AppState {
    pub fn new() -> Self {
        let config = Config::load();
        let date_order = match config.fecha_orden.to_lowercase().as_str() {
            "mdy" => DateOrder::Mdy,
            _ => DateOrder::Dmy,
        };

        let autosave = AutosaveManager::new(config.autosave_ms);
        let clipboard = ClipboardManager::new();

        let mut state = Self {
            config,
            workspace: None,
            watcher: None,
            open_notes: Vec::new(),
            active_note_idx: None,
            autosave,
            clipboard,
            search_index: SearchIndex::new(),
            palette_open: false,
            palette_state: PaletteState::default(),
            editor_state: EditorUiState::default(),
            sidebar_visible: true,
            toast: None,
            date_order,
            new_note_dialog: None,
            new_note_input: String::new(),
            new_project_dialog: false,
            new_project_input: String::new(),
            dialog_just_opened: false,
        };

        // Try saved workspace first
        if let Some(ws_path) = state.config.workspace.clone() {
            if ws_path.exists() {
                state.set_workspace(&ws_path);
            }
        }

        // If no workspace yet, ensure a default one
        if state.workspace.is_none() {
            let _ = state.ensure_workspace();
        }

        // Auto-open first note if available
        if state.open_notes.is_empty() {
            if let Some(ws) = &state.workspace {
                let first_path = ws.projects
                    .first()
                    .and_then(|p| p.notes.first())
                    .map(|n| n.path.clone())
                    .or_else(|| ws.general.first().map(|n| n.path.clone()));

                if let Some(p) = first_path {
                    state.open_note(&p);
                }
            }
        }

        state
    }

    pub fn ensure_workspace(&mut self) -> Result<PathBuf, String> {
        if let Some(ws) = &self.workspace {
            return Ok(ws.root.clone());
        }

        // 1. Check if local `ejemplo_workspace` exists
        if let Ok(cur) = std::env::current_dir() {
            let sample = cur.join("ejemplo_workspace");
            if sample.exists() {
                self.set_workspace(&sample);
                return Ok(sample);
            }
        }

        // 2. Default to MisNotas in current dir
        let default_root = if let Ok(cur) = std::env::current_dir() {
            cur.join("MisNotas")
        } else {
            PathBuf::from("MisNotas")
        };

        if let Err(e) = std::fs::create_dir_all(&default_root) {
            return Err(format!("No se pudo inicializar carpeta de notas: {}", e));
        }

        self.set_workspace(&default_root);
        Ok(default_root)
    }

    pub fn set_workspace(&mut self, root: &Path) {
        if let Ok(ws) = scan_workspace(root) {
            self.search_index.build_from_workspace(&ws);
            self.watcher = WorkspaceWatcher::new(root).ok();
            self.workspace = Some(ws);
            self.config.workspace = Some(root.to_path_buf());
            let _ = self.config.save();
        }
    }

    pub fn rescan_workspace(&mut self) {
        if let Some(ws) = &self.workspace {
            let root = ws.root.clone();
            if let Ok(new_ws) = scan_workspace(&root) {
                self.search_index.build_from_workspace(&new_ws);
                self.workspace = Some(new_ws);
            }
        }
    }

    pub fn open_note(&mut self, path: &Path) {
        let norm_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        // If already open, switch to it
        if let Some(idx) = self.open_notes.iter().position(|n| {
            n.path == path || n.path.canonicalize().ok() == Some(norm_path.clone())
        }) {
            self.active_note_idx = Some(idx);
            return;
        }

        // Otherwise load from disk
        match load_note(path) {
            Ok(note) => {
                self.open_notes.push(note);
                self.active_note_idx = Some(self.open_notes.len() - 1);
            }
            Err(e) => {
                self.show_toast(format!("Error al abrir nota: {}", e));
            }
        }
    }

    pub fn save_active_note(&mut self) {
        if let Some(idx) = self.active_note_idx {
            if let Some(note) = self.open_notes.get_mut(idx) {
                if note.dirty {
                    match save_note(note) {
                        Ok(_) => {
                            self.autosave.remove(&note.path);
                            self.show_toast("Nota guardada".to_string());
                        }
                        Err(e) => {
                            self.show_toast(format!("Error al guardar: {}", e));
                        }
                    }
                }
            }
        }
    }

    pub fn close_tab(&mut self, idx: usize) {
        if idx < self.open_notes.len() {
            let mut note = self.open_notes.remove(idx);
            if note.dirty {
                let _ = save_note(&mut note);
            }

            if self.open_notes.is_empty() {
                self.active_note_idx = None;
            } else if let Some(active) = self.active_note_idx {
                if active >= self.open_notes.len() {
                    self.active_note_idx = Some(self.open_notes.len() - 1);
                }
            }
        }
    }

    pub fn create_and_open_note(&mut self, project: Option<&str>, title: &str) {
        let root = match self.ensure_workspace() {
            Ok(r) => r,
            Err(e) => {
                self.show_toast(e);
                return;
            }
        };

        match create_note(&root, project, title) {
            Ok(note_path) => {
                self.rescan_workspace();
                self.open_note(&note_path);
                self.show_toast(format!("Nota \"{}\" creada", title));
            }
            Err(e) => {
                self.show_toast(format!("Error al crear nota: {}", e));
            }
        }
    }

    pub fn create_new_project(&mut self, name: &str) {
        let root = match self.ensure_workspace() {
            Ok(r) => r,
            Err(e) => {
                self.show_toast(e);
                return;
            }
        };

        match create_project(&root, name) {
            Ok(_) => {
                self.rescan_workspace();
                self.show_toast(format!("Proyecto \"{}\" creado", name));
            }
            Err(e) => {
                self.show_toast(format!("Error al crear proyecto: {}", e));
            }
        }
    }

    pub fn show_toast(&mut self, msg: String) {
        self.toast = Some((msg, Instant::now()));
    }

    pub fn update(&mut self) {
        self.clipboard.update();

        // Check toast expiration (3 seconds)
        if let Some((_, time)) = &self.toast {
            if time.elapsed().as_secs() >= 3 {
                self.toast = None;
            }
        }

        // Check autosave
        for note in &mut self.open_notes {
            if note.dirty && self.autosave.check_should_save(&note.path) {
                if save_note(note).is_ok() {
                    self.autosave.remove(&note.path);
                }
            }
        }

        // Check watcher events
        if let Some(watcher) = &self.watcher {
            while let Ok(event) = watcher.rx.try_recv() {
                if let WorkspaceEvent::Changed(path) = event {
                    // If modified externally and we don't have local edits, reload
                    for note in &mut self.open_notes {
                        if note.path == path && !note.dirty {
                            if let Ok(reloaded) = load_note(&path) {
                                *note = reloaded;
                            }
                        }
                    }
                }
            }
        }
    }
}
