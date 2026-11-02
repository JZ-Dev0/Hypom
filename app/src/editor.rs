use std::collections::HashMap;
use egui::{Key, Modifiers, RichText, ScrollArea, Ui};
use hypom_core::dates::DateOrder;
use hypom_core::parse::Segment;
use hypom_storage::LoadedNote;
use crate::block_view::{render_block_segment, BlockUiState};
use crate::clipboard::ClipboardManager;
use crate::text_segment::render_text_segment;

pub struct EditorUiState {
    pub block_states: HashMap<usize, BlockUiState>,
}

impl Default for EditorUiState {
    fn default() -> Self {
        Self {
            block_states: HashMap::new(),
        }
    }
}

pub fn render_editor(
    ui: &mut Ui,
    note: &mut LoadedNote,
    editor_state: &mut EditorUiState,
    clipboard: &mut ClipboardManager,
    clipboard_clear_secs: u64,
    date_order: &DateOrder,
    toast_msg: &mut Option<String>,
) -> bool {
    let mut modified = false;

    // Check Ctrl+Enter to insert a new block template
    if ui.input(|i| i.key_pressed(Key::Enter) && i.modifiers.contains(Modifiers::COMMAND | Modifiers::CTRL)) {
        let block_template = "\n@tabla \"Nueva tabla\" item, valor\nEjemplo 100\n@fin\n";
        let mut full = note.full_text();
        full.push_str(block_template);
        note.segments = hypom_core::parse::parse_note(&full);
        note.dirty = true;
        *toast_msg = Some("Nuevo bloque insertado".to_string());
        return true;
    }

    ScrollArea::vertical()
        .id_salt("editor_scroll_area")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(8.0);

            let segment_count = note.segments.len();
            for seg_idx in 0..segment_count {
                ui.push_id(seg_idx, |ui| {
                    match &mut note.segments[seg_idx] {
                        Segment::Text(t) => {
                            if render_text_segment(ui, t) {
                                modified = true;
                            }
                        }
                        Segment::Block(b) => {
                            let block_ui_state = editor_state.block_states.entry(seg_idx).or_default();
                            if render_block_segment(
                                ui,
                                b,
                                block_ui_state,
                                clipboard,
                                clipboard_clear_secs,
                                date_order,
                                toast_msg,
                            ) {
                                modified = true;
                            }
                        }
                    }
                });
                ui.add_space(8.0);
            }

            if note.segments.is_empty() {
                ui.label(RichText::new("Nota vacía. Escribe texto o usa Ctrl+Enter para insertar un bloque.").weak().italics());
            }
        });

    if modified {
        note.dirty = true;
    }

    modified
}
