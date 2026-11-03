use egui::{Color32, RichText, ScrollArea, Ui};
use hypom_storage::LoadedNote;
use crate::icons;

pub struct TabAction {
    pub select_tab: Option<usize>,
    pub close_tab: Option<usize>,
    pub new_note: bool,
}

pub fn render_tabs(
    ui: &mut Ui,
    open_notes: &[LoadedNote],
    active_idx: Option<usize>,
) -> TabAction {
    let mut action = TabAction {
        select_tab: None,
        close_tab: None,
        new_note: false,
    };

    ui.set_min_width(ui.available_width());

    ui.horizontal(|ui| {
        // Tabs scroll horizontally when the window is narrow or there are many open notes
        ScrollArea::horizontal()
            .id_salt("tabs_scroll_area")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (i, note) in open_notes.iter().enumerate() {
                        let is_active = active_idx == Some(i);
                        let mut title = note.title();
                        if note.dirty {
                            title.push('•');
                        }

                        // Truncate very long titles so tabs stay usable
                        let display = if title.chars().count() > 28 {
                            let truncated: String = title.chars().take(26).collect();
                            format!("{}…", truncated)
                        } else {
                            title.clone()
                        };

                        let text = if is_active {
                            RichText::new(&display)
                                .strong()
                                .color(Color32::from_rgb(100, 180, 255))
                        } else {
                            RichText::new(&display)
                        };

                        ui.push_id(i, |ui| {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    let tab = ui
                                        .selectable_label(is_active, text)
                                        .on_hover_text(&title);
                                    if tab.clicked() {
                                        action.select_tab = Some(i);
                                    }

                                    if ui
                                        .small_button(icons::X)
                                        .on_hover_text("Cerrar")
                                        .clicked()
                                    {
                                        action.close_tab = Some(i);
                                    }
                                });
                            });
                        });
                    }
                });
            });

        if ui
            .button(icons::PLUS)
            .on_hover_text("Nueva nota")
            .clicked()
        {
            action.new_note = true;
        }
    });

    action
}
