use egui::{Color32, RichText, Ui};
use hypom_storage::LoadedNote;

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

    ui.horizontal(|ui| {
        for (i, note) in open_notes.iter().enumerate() {
            let is_active = active_idx == Some(i);
            let mut title = note.title();
            if note.dirty {
                title.push('•');
            }

            let text = if is_active {
                RichText::new(&title).strong().color(Color32::from_rgb(100, 180, 255))
            } else {
                RichText::new(&title)
            };

            ui.push_id(i, |ui| {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        if ui.selectable_label(is_active, text).clicked() {
                            action.select_tab = Some(i);
                        }

                        if ui.small_button("✕").clicked() {
                            action.close_tab = Some(i);
                        }
                    });
                });
            });
        }

        if ui.button("+").on_hover_text("Nueva nota").clicked() {
            action.new_note = true;
        }
    });

    action
}

