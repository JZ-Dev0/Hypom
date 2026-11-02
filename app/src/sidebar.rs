use std::path::{Path, PathBuf};
use egui::{Color32, RichText, ScrollArea, Ui};
use hypom_storage::Workspace;

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

    ui.vertical(|ui| {
        // Workspace / Root info
        ui.horizontal(|ui| {
            if let Some(ws) = workspace {
                let folder_name = ws.root
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Workspace".to_string());
                ui.label(RichText::new(format!("📁 {}", folder_name)).strong().size(15.0));
            } else {
                ui.label(RichText::new("📁 Sin espacio").strong().size(15.0));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("📂").on_hover_text("Cambiar carpeta del espacio de trabajo").clicked() {
                    action = Some(SidebarAction::ChooseWorkspace);
                }
            });
        });

        ui.add_space(6.0);

        // Search shortcut button (Ctrl+P)
        if ui.button("🔍 Buscar (Ctrl+P)").clicked() {
            action = Some(SidebarAction::OpenPalette);
        }

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui.button("+ Nota").clicked() {
                action = Some(SidebarAction::NewNote(None));
            }
            if ui.button("+ Proyecto").clicked() {
                action = Some(SidebarAction::NewProject);
            }
        });

        ui.separator();

        let ws = match workspace {
            Some(w) => w,
            None => {
                ui.label(RichText::new("Selecciona una carpeta para comenzar").weak());
                if ui.button("📂 Abrir carpeta").clicked() {
                    action = Some(SidebarAction::ChooseWorkspace);
                }
                return;
            }
        };

        ScrollArea::vertical()
            .id_salt("sidebar_scroll_area")
            .show(ui, |ui| {
                // General notes
                egui::CollapsingHeader::new(RichText::new("▾ General").strong())
                    .id_salt("general_collapsing_header")
                    .default_open(true)
                    .show(ui, |ui| {
                        if ws.general.is_empty() {
                            ui.label(RichText::new("Sin notas").weak().italics());
                        } else {
                            for note in &ws.general {
                                ui.push_id(&note.path, |ui| {
                                    let is_selected = active_note_path == Some(&note.path);
                                    let text = if is_selected {
                                        RichText::new(&note.title).strong().color(Color32::from_rgb(100, 180, 255))
                                    } else {
                                        RichText::new(&note.title)
                                    };

                                    if ui.selectable_label(is_selected, text).clicked() {
                                        action = Some(SidebarAction::OpenNote(note.path.clone()));
                                    }
                                });
                            }
                        }
                    });

                ui.add_space(4.0);

                // Projects
                for project in &ws.projects {
                    let display = project.display_name();
                    let icon = project.meta.icono.as_deref().unwrap_or("▾");
                    let label = format!("{} {}", icon, display);

                    egui::CollapsingHeader::new(RichText::new(label).strong())
                        .id_salt(format!("project_header_{}", project.name))
                        .default_open(true)
                        .show(ui, |ui| {
                            if project.notes.is_empty() {
                                ui.label(RichText::new("Sin notas").weak().italics());
                            } else {
                                for note in &project.notes {
                                    ui.push_id(&note.path, |ui| {
                                        let is_selected = active_note_path == Some(&note.path);
                                        let text = if is_selected {
                                            RichText::new(&note.title).strong().color(Color32::from_rgb(100, 180, 255))
                                        } else {
                                            RichText::new(&note.title)
                                        };

                                        if ui.selectable_label(is_selected, text).clicked() {
                                            action = Some(SidebarAction::OpenNote(note.path.clone()));
                                        }
                                    });
                                }
                            }

                            if ui.small_button("+ nota").clicked() {
                                action = Some(SidebarAction::NewNote(Some(project.name.clone())));
                            }
                        });
                }
            });
    });

    action
}
