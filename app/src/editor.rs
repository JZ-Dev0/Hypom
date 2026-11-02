use std::collections::HashMap;
use egui::{Key, Modifiers, RichText, ScrollArea, Ui};
use hypom_core::dates::DateOrder;
use hypom_core::parse::Segment;
use hypom_storage::LoadedNote;
use crate::block_view::{render_block_segment, sync_raw_from_block, BlockUiState};
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
    let mut should_reparse = false;
    let enter_pressed = ui.input(|i| i.key_pressed(Key::Enter));

    // Check Ctrl+Enter to insert a new block template
    if enter_pressed && ui.input(|i| i.modifiers.contains(Modifiers::COMMAND | Modifiers::CTRL)) {
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
                                if check_and_auto_close_block(&mut t.text, enter_pressed) {
                                    should_reparse = true;
                                }
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

    if should_reparse {
        note.reparse_all();
        // If a newly created block has no rows, add an initial empty row and focus cell 0
        for (idx, seg) in note.segments.iter_mut().enumerate() {
            if let Segment::Block(b) = seg {
                if b.parsed.rows.is_empty() && !b.parsed.schema.fields.is_empty() {
                    hypom_core::ops::add_row(&mut b.parsed);
                    b.raw = sync_raw_from_block(&b.parsed);
                    let state = editor_state.block_states.entry(idx).or_default();
                    state.editing_cell = Some((0, 0));
                    state.edit_buffer = String::new();
                    state.focus_edit = true;
                }
            }
        }
        note.dirty = true;
        modified = true;
        *toast_msg = Some("Tabla creada".to_string());
    }

    if modified {
        note.dirty = true;
    }

    modified
}

/// Checks if text contains an unclosed @tabla or @registro block that should be auto-closed and parsed
fn check_and_auto_close_block(text: &mut String, enter_pressed: bool) -> bool {
    if !enter_pressed && !text.contains("@fin") {
        return false;
    }

    let lines: Vec<&str> = text.split('\n').collect();
    let mut header_idx = None;
    let mut has_fin = false;

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("@tabla") || trimmed.starts_with("@registro") {
            header_idx = Some(idx);
        } else if trimmed == "@fin" {
            has_fin = true;
        }
    }

    let h_idx = match header_idx {
        Some(idx) => idx,
        None => return false,
    };

    if has_fin {
        return true;
    }

    if enter_pressed {
        let mut new_lines = Vec::new();
        let mut inserted_fin = false;

        for (idx, line) in lines.iter().enumerate() {
            new_lines.push(line.to_string());
            if idx >= h_idx && !inserted_fin {
                if idx + 1 >= lines.len() || lines[idx + 1].trim().is_empty() {
                    new_lines.push("@fin".to_string());
                    inserted_fin = true;
                }
            }
        }

        if !inserted_fin {
            new_lines.push("@fin".to_string());
        }

        *text = new_lines.join("\n");
        return true;
    }

    false
}
