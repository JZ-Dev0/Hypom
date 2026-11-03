//! Phosphor icon aliases used across the Hypom UI.
//! Prefer these over emoji so glyphs render consistently via egui-phosphor.

pub use egui_phosphor::regular::{
    APPROXIMATE_EQUALS, ARROW_DOWN, ARROW_UP, CARET_DOWN, CARET_RIGHT, CLIPBOARD_TEXT, CLOCK, CODE,
    FILE_TEXT, FOLDER, FOLDER_DASHED, FOLDER_OPEN, FOLDER_PLUS, FUNNEL, HARD_DRIVES, INFO,
    LIST_CHECKS, LOCK, MAGNIFYING_GLASS, NOTE_PENCIL, PLUS, ROWS_PLUS_BOTTOM, SIDEBAR,
    SIDEBAR_SIMPLE, SIGMA, TABLE, TRASH, WARNING, X,
};

/// Resolve a `proyecto.toml` `icono` value to a Phosphor glyph when possible.
/// Accepts aliases (`server`, `folder`, …) and legacy emoji → Phosphor mapping.
pub fn resolve_project_icon(raw: Option<&str>) -> &'static str {
    let Some(s) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return FOLDER;
    };

    match s.to_lowercase().as_str() {
        "folder" | "carpeta" => FOLDER,
        "server" | "servidor" | "servers" | "hard-drives" | "hdd" => HARD_DRIVES,
        "note" | "nota" | "notes" => NOTE_PENCIL,
        "list" | "lista" | "checks" => LIST_CHECKS,
        "lock" | "secreto" | "secure" => LOCK,
        "search" | "buscar" => MAGNIFYING_GLASS,
        "table" | "tabla" => TABLE,
        "info" => INFO,
        // Legacy emoji from older proyecto.toml → Phosphor
        "🖥" | "💻" | "🖥️" => HARD_DRIVES,
        "📁" | "📂" => FOLDER,
        "🔒" => LOCK,
        "📝" | "📄" => NOTE_PENCIL,
        _ => FOLDER,
    }
}
