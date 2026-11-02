pub mod config;
pub mod workspace;
pub mod note;
pub mod autosave;
pub mod watcher;
pub mod index;

pub use config::Config;
pub use workspace::{Workspace, Project, ProjectMeta, NoteRef, scan_workspace, create_note, create_project};
pub use note::{LoadedNote, load_note, save_note};
pub use autosave::AutosaveManager;
pub use watcher::{WorkspaceWatcher, WorkspaceEvent};
pub use index::{SearchIndex, IndexEntry};
