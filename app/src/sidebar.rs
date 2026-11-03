use std::path::{Path, PathBuf};
use egui::{Color32, RichText, ScrollArea, Ui};
use hypom_storage::Workspace;
use crate::icons;

pub enum SidebarAction {
    OpenNote(PathBuf),
    NewNote(Option<String>), // None = General, Some(project_folder)
    NewProject,
    ChooseWorkspace,
    OpenPalette,
}

pub fn render_sidebar(
    ui: &mut Ui,
    workspace: Option<&Workspace>,
    active_note_path: Option<&Path>,
) -> Option<SidebarAction> {
    let mut action = None;

    // Fill the SidePanel so contents reflow with panel width
    ui.set_min_width(ui.available_width());
    ui.set_min_height(ui.available_height());

    ui.vertical(|ui| {
        ui.set_min_width(ui.available_width());

        // Workspace / Root info
        ui.horizontal_wrapped(|ui| {
            if let Some(ws) = workspace {
                let folder_name = ws
                    .root
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Workspace".to_string());
                ui.label(
                    RichText::new(format!("{} {}", icons::FOLDER, folder_name))
                        .strong()
                        .size(15.0),
                );
            } else {
                ui.label(
                    RichText::new(format!("{} Sin espacio", icons::FOLDER_DASHED))
                        .strong()
                        .size(15.0),
                );
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(icons::FOLDER_OPEN)
                    .on_hover_text("Cambiar carpeta del espacio de trabajo")
                    .clicked()
                {
                    action = Some(SidebarAction::ChooseWorkspace);
                }
            });
        });

        ui.add_space(6.0);

        if ui
            .add_sized(
                [ui.available_width(), 24.0],
                egui::Button::new(format!("{} Buscar (Ctrl+P)", icons::MAGNIFYING_GLASS)),
            )
            .clicked()
        {
            action = Some(SidebarAction::OpenPalette);
        }

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            let half = (ui.available_width() - ui.spacing().item_spacing.x).max(40.0) / 2.0;
            if ui
                .add_sized(
                    [half, 22.0],
                    egui::Button::new(format!("{} Nota", icons::NOTE_PENCIL)),
                )
                .clicked()
            {
                action = Some(SidebarAction::NewNote(None));
            }
            if ui
                .add_sized(
                    [half, 22.0],
                    egui::Button::new(format!("{} Proyecto", icons::FOLDER_PLUS)),
                )
                .clicked()
            {
                action = Some(SidebarAction::NewProject);
            }
        });

        ui.separator();

        let ws = match workspace {
            Some(w) => w,
            None => {
                ui.label(RichText::new("Selecciona una carpeta para comenzar").weak());
                if ui
                    .button(format!("{} Abrir carpeta", icons::FOLDER_OPEN))
                    .clicked()
                {
                    action = Some(SidebarAction::ChooseWorkspace);
                }
                return;
            }
        };

        // Take the remaining vertical space and scroll when the tree is tall
        ScrollArea::vertical()
            .id_salt("sidebar_scroll_area")
            .auto_shrink([false, false])
            .max_height(ui.available_height())
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                egui::CollapsingHeader::new(
                    RichText::new(format!("{} General", icons::FILE_TEXT)).strong(),
                )
                    .id_salt("general_collapsing_header")
                    .default_open(true)
                    .show(ui, |ui| {
                        if ws.general.is_empty() {
                            ui.label(RichText::new("Sin notas").weak().italics());
                        } else {
                            for note in &ws.general {
                                render_note_item(ui, note.path.as_path(), &note.title, active_note_path, &mut action);
                            }
                        }
                    });

                ui.add_space(4.0);

                for project in &ws.projects {
                    let display = project.display_name();
                    let icon = icons::resolve_project_icon(project.meta.icono.as_deref());
                    let label = format!("{} {}", icon, display);

                    egui::CollapsingHeader::new(RichText::new(label).strong())
                        .id_salt(format!("project_header_{}", project.name))
                        .default_open(true)
                        .show(ui, |ui| {
                            if project.notes.is_empty() {
                                ui.label(RichText::new("Sin notas").weak().italics());
                            } else {
                                for note in &project.notes {
                                    render_note_item(
                                        ui,
                                        note.path.as_path(),
                                        &note.title,
                                        active_note_path,
                                        &mut action,
                                    );
                                }
                            }

                            if ui
                                .small_button(format!("{} nota", icons::PLUS))
                                .clicked()
                            {
                                action = Some(SidebarAction::NewNote(Some(project.name.clone())));
                            }
                        });
                }
            });
    });

    action
}

fn render_note_item(
    ui: &mut Ui,
    path: &Path,
    title: &str,
    active_note_path: Option<&Path>,
    action: &mut Option<SidebarAction>,
) {
    ui.push_id(path, |ui| {
        let is_selected = active_note_path == Some(path);
        let note_label = format!("{} {}", icons::FILE_TEXT, title);
        let text = if is_selected {
            RichText::new(note_label)
                .strong()
                .color(Color32::from_rgb(100, 180, 255))
        } else {
            RichText::new(note_label)
        };

        let resp = ui
            .add_sized(
                [ui.available_width(), 18.0],
                egui::SelectableLabel::new(is_selected, text),
            )
            .on_hover_text(title);

        if resp.clicked() {
            *action = Some(SidebarAction::OpenNote(path.to_path_buf()));
        }
    });
}
