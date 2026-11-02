use std::path::PathBuf;
use egui::{Color32, Frame, Key, Modifiers, RichText, ScrollArea, Stroke, TextEdit, Window};
use hypom_storage::{IndexEntry, SearchIndex};
use crate::clipboard::ClipboardManager;

pub enum PaletteAction {
    OpenNote(PathBuf),
    Close,
}

pub struct PaletteState {
    pub query: String,
    pub selected_idx: usize,
}

impl Default for PaletteState {
    fn default() -> Self {
        Self {
            query: String::new(),
            selected_idx: 0,
        }
    }
}

pub fn render_palette(
    ctx: &egui::Context,
    state: &mut PaletteState,
    index: &SearchIndex,
    clipboard: &mut ClipboardManager,
    toast_msg: &mut Option<String>,
) -> Option<PaletteAction> {
    let mut action = None;

    Window::new("Buscar (Ctrl+P)")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_TOP, [0.0, 80.0])
        .fixed_size([550.0, 380.0])
        .frame(
            Frame::window(&ctx.style())
                .fill(Color32::from_rgb(26, 29, 36))
                .stroke(Stroke::new(1.0, Color32::from_rgb(70, 78, 96))),
        )
        .show(ctx, |ui| {
            // Search query input
            let edit_resp = ui.add(
                TextEdit::singleline(&mut state.query)
                    .hint_text("Buscar notas o registros...")
                    .desired_width(f32::INFINITY),
            );

            // Focus search box automatically
            edit_resp.request_focus();

            ui.add_space(8.0);

            // Filter entries
            let query_lower = state.query.to_lowercase();
            let matches: Vec<&IndexEntry> = index
                .entries
                .iter()
                .filter(|entry| {
                    if query_lower.is_empty() {
                        !entry.is_row // show notes when empty
                    } else {
                        entry.text.to_lowercase().contains(&query_lower)
                            || entry.note_title.to_lowercase().contains(&query_lower)
                    }
                })
                .take(30)
                .collect();

            // Keyboard navigation
            if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
                if !matches.is_empty() {
                    state.selected_idx = (state.selected_idx + 1) % matches.len();
                }
            }
            if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
                if !matches.is_empty() {
                    if state.selected_idx == 0 {
                        state.selected_idx = matches.len() - 1;
                    } else {
                        state.selected_idx -= 1;
                    }
                }
            }
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                action = Some(PaletteAction::Close);
            }

            let enter_pressed = ctx.input(|i| i.key_pressed(Key::Enter));
            let ctrl_enter = enter_pressed && ctx.input(|i| i.modifiers.contains(Modifiers::COMMAND | Modifiers::CTRL));

            ScrollArea::vertical()
                .id_salt("palette_scroll_area")
                .max_height(300.0)
                .show(ui, |ui| {
                if matches.is_empty() {
                    ui.label(RichText::new("No se encontraron resultados").weak().italics());
                    return;
                }

                for (idx, entry) in matches.iter().enumerate() {
                    ui.push_id(idx, |ui| {
                        let is_selected = state.selected_idx == idx;

                        let bg_color = if is_selected {
                            Color32::from_rgb(45, 52, 70)
                        } else {
                            Color32::TRANSPARENT
                        };

                        Frame::new().fill(bg_color).inner_margin(4.0).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if entry.is_row {
                                    ui.label(RichText::new("📋").size(14.0));
                                    ui.vertical(|ui| {
                                        let title = entry.block_title.as_deref().unwrap_or("Registro");
                                        ui.label(RichText::new(format!("{} en {}", title, entry.note_title)).size(11.0).weak());
                                        ui.label(RichText::new(&entry.text).strong());
                                    });
                                } else {
                                    ui.label(RichText::new("📄").size(14.0));
                                    ui.vertical(|ui| {
                                        let proj = entry.project_name.as_deref().unwrap_or("General");
                                        ui.label(RichText::new(proj).size(11.0).weak());
                                        ui.label(RichText::new(&entry.note_title).strong());
                                    });
                                }
                            });
                        });

                        if is_selected {
                            if ctrl_enter && entry.is_row {
                                let text = entry.row_cells.join("\t");
                                clipboard.copy_text(&text);
                                *toast_msg = Some("Registro copiado directamente".to_string());
                                action = Some(PaletteAction::Close);
                            } else if enter_pressed {
                                action = Some(PaletteAction::OpenNote(entry.note_path.clone()));
                            }
                        }
                    });
                }
            });
        });

    action
}
