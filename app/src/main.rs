#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod block_view;
mod clipboard;
mod editor;
mod palette;
mod sidebar;
mod state;
mod tabs;
mod text_segment;
mod theme;

use eframe::egui::{self, Color32, Key, Modifiers, RichText, TopBottomPanel, CentralPanel, SidePanel};
use state::AppState;
use sidebar::{render_sidebar, SidebarAction};
use tabs::render_tabs;
use editor::render_editor;
use palette::{render_palette, PaletteAction};

struct HypomApp {
    state: AppState,
}

impl HypomApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            state: AppState::new(),
        }
    }
}

impl eframe::App for HypomApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.state.update();

        // Handle Global Shortcuts
        if ctx.input(|i| i.key_pressed(Key::S) && i.modifiers.contains(Modifiers::COMMAND | Modifiers::CTRL)) {
            self.state.save_active_note();
        }

        if ctx.input(|i| i.key_pressed(Key::P) && i.modifiers.contains(Modifiers::COMMAND | Modifiers::CTRL)) {
            self.state.palette_open = !self.state.palette_open;
            if self.state.palette_open {
                self.state.palette_state.query.clear();
            }
        }

        if ctx.input(|i| i.key_pressed(Key::B) && i.modifiers.contains(Modifiers::COMMAND | Modifiers::CTRL)) {
            self.state.sidebar_visible = !self.state.sidebar_visible;
        }

        if ctx.input(|i| i.key_pressed(Key::W) && i.modifiers.contains(Modifiers::COMMAND | Modifiers::CTRL)) {
            if let Some(active) = self.state.active_note_idx {
                self.state.close_tab(active);
            }
        }

        if ctx.input(|i| i.key_pressed(Key::N) && i.modifiers.contains(Modifiers::COMMAND | Modifiers::CTRL)) {
            self.state.new_note_dialog = Some(None);
            self.state.new_note_input.clear();
            self.state.dialog_just_opened = true;
        }

        if ctx.input(|i| i.key_pressed(Key::Tab) && i.modifiers.contains(Modifiers::COMMAND | Modifiers::CTRL)) {
            if !self.state.open_notes.is_empty() {
                let current = self.state.active_note_idx.unwrap_or(0);
                if ctx.input(|i| i.modifiers.shift) {
                    let prev = if current == 0 { self.state.open_notes.len() - 1 } else { current - 1 };
                    self.state.active_note_idx = Some(prev);
                } else {
                    let next = (current + 1) % self.state.open_notes.len();
                    self.state.active_note_idx = Some(next);
                }
            }
        }

        // Top bar (tabs)
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            let active_idx = self.state.active_note_idx;
            let tab_action = render_tabs(ui, &self.state.open_notes, active_idx);

            if let Some(idx) = tab_action.select_tab {
                self.state.active_note_idx = Some(idx);
            }
            if let Some(idx) = tab_action.close_tab {
                self.state.close_tab(idx);
            }
            if tab_action.new_note {
                self.state.new_note_dialog = Some(None);
                self.state.new_note_input.clear();
                self.state.dialog_just_opened = true;
            }
        });

        // Bottom status bar
        TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some((msg, _)) = &self.state.toast {
                    ui.label(RichText::new(format!("ℹ {}", msg)).color(Color32::from_rgb(130, 220, 150)));
                } else if let Some(idx) = self.state.active_note_idx {
                    if let Some(note) = self.state.open_notes.get(idx) {
                        let eol_str = match note.eol {
                            hypom_core::parse::Eol::Lf => "LF",
                            hypom_core::parse::Eol::CrLf => "CRLF",
                        };
                        let dirty_str = if note.dirty { "Modificado" } else { "Guardado" };
                        ui.label(RichText::new(format!("{} | {} | UTF-8", dirty_str, eol_str)).weak().size(12.0));
                    }
                } else {
                    ui.label(RichText::new("Hypom — Bloc de notas con bloques estructurados").weak().size(12.0));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button(if self.state.sidebar_visible { "◨ Barra lateral" } else { "◧ Barra lateral" }).clicked() {
                        self.state.sidebar_visible = !self.state.sidebar_visible;
                    }
                });
            });
        });

        // Sidebar
        if self.state.sidebar_visible {
            SidePanel::left("sidebar_panel")
                .default_width(220.0)
                .width_range(160.0..=360.0)
                .show(ctx, |ui| {
                    let active_path = self.state.active_note_idx
                        .and_then(|idx| self.state.open_notes.get(idx))
                        .map(|n| n.path.as_path());

                    if let Some(action) = render_sidebar(ui, self.state.workspace.as_ref(), active_path) {
                        match action {
                            SidebarAction::OpenNote(path) => {
                                self.state.open_note(&path);
                            }
                            SidebarAction::NewNote(project_folder) => {
                                self.state.new_note_dialog = Some(project_folder);
                                self.state.new_note_input.clear();
                                self.state.dialog_just_opened = true;
                            }
                            SidebarAction::NewProject => {
                                self.state.new_project_dialog = true;
                                self.state.new_project_input.clear();
                                self.state.dialog_just_opened = true;
                            }
                            SidebarAction::ChooseWorkspace => {
                                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                    self.state.set_workspace(&folder);
                                }
                            }
                            SidebarAction::OpenPalette => {
                                self.state.palette_open = true;
                                self.state.palette_state.query.clear();
                            }
                        }
                    }
                });
        }

        // Central editor panel
        CentralPanel::default().show(ctx, |ui| {
            if let Some(active_idx) = self.state.active_note_idx {
                if let Some(note) = self.state.open_notes.get_mut(active_idx) {
                    let mut toast_msg = None;
                    let path_clone = note.path.clone();

                    let modified = render_editor(
                        ui,
                        note,
                        &mut self.state.editor_state,
                        &mut self.state.clipboard,
                        self.state.config.clipboard_clear,
                        &self.state.date_order,
                        &mut toast_msg,
                    );

                    if modified {
                        self.state.autosave.mark_modified(path_clone);
                    }

                    if let Some(msg) = toast_msg {
                        self.state.show_toast(msg);
                    }
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Hypom");
                        ui.add_space(8.0);
                        ui.label(RichText::new("Abre una nota desde la barra lateral o crea una nueva con Ctrl+N").weak());
                        ui.add_space(12.0);
                        if ui.button("+ Nueva Nota").clicked() {
                            self.state.new_note_dialog = Some(None);
                            self.state.new_note_input.clear();
                            self.state.dialog_just_opened = true;
                        }
                    });
                });
            }
        });

        // Command Palette Modal (Ctrl+P)
        if self.state.palette_open {
            let mut toast_msg = None;
            if let Some(palette_action) = render_palette(
                ctx,
                &mut self.state.palette_state,
                &self.state.search_index,
                &mut self.state.clipboard,
                &mut toast_msg,
            ) {
                match palette_action {
                    PaletteAction::OpenNote(path) => {
                        self.state.open_note(&path);
                        self.state.palette_open = false;
                    }
                    PaletteAction::Close => {
                        self.state.palette_open = false;
                    }
                }
            }

            if let Some(msg) = toast_msg {
                self.state.show_toast(msg);
            }
        }

        // New Note Dialog Modal
        if let Some(project_folder) = self.state.new_note_dialog.clone() {
            let mut close_dialog = false;
            let mut create_confirmed = false;

            egui::Window::new("Nueva Nota")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .fixed_size([320.0, 130.0])
                .show(ctx, |ui| {
                    let target_info = if let Some(p) = &project_folder {
                        format!("Crear nota en proyecto: {}", p)
                    } else {
                        "Crear nota en General".to_string()
                    };
                    ui.label(RichText::new(target_info).weak().size(12.0));
                    ui.add_space(4.0);

                    ui.label("Nombre del archivo (ej. ideas.txt o ideas):");
                    let resp = ui.text_edit_singleline(&mut self.state.new_note_input);
                    if self.state.dialog_just_opened {
                        resp.request_focus();
                        self.state.dialog_just_opened = false;
                    }

                    if (resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)))
                        || ui.input(|i| i.key_pressed(Key::Enter))
                    {
                        create_confirmed = true;
                    }
                    if ui.input(|i| i.key_pressed(Key::Escape)) {
                        close_dialog = true;
                    }

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Crear").clicked() {
                            create_confirmed = true;
                        }
                        if ui.button("Cancelar").clicked() {
                            close_dialog = true;
                        }
                    });
                });

            if create_confirmed {
                let name = self.state.new_note_input.trim().to_string();
                if name.is_empty() {
                    self.state.show_toast("Por favor escribe un nombre para la nota".to_string());
                } else {
                    self.state.create_and_open_note(project_folder.as_deref(), &name);
                    self.state.new_note_dialog = None;
                }
            } else if close_dialog {
                self.state.new_note_dialog = None;
            }
        }

        // New Project Dialog Modal
        if self.state.new_project_dialog {
            let mut close_dialog = false;
            let mut create_confirmed = false;

            egui::Window::new("Nuevo Proyecto")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .fixed_size([320.0, 130.0])
                .show(ctx, |ui| {
                    ui.label("Nombre de la carpeta del proyecto:");
                    let resp = ui.text_edit_singleline(&mut self.state.new_project_input);
                    if self.state.dialog_just_opened {
                        resp.request_focus();
                        self.state.dialog_just_opened = false;
                    }

                    if (resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)))
                        || ui.input(|i| i.key_pressed(Key::Enter))
                    {
                        create_confirmed = true;
                    }
                    if ui.input(|i| i.key_pressed(Key::Escape)) {
                        close_dialog = true;
                    }

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Crear").clicked() {
                            create_confirmed = true;
                        }
                        if ui.button("Cancelar").clicked() {
                            close_dialog = true;
                        }
                    });
                });

            if create_confirmed {
                let name = self.state.new_project_input.trim().to_string();
                if name.is_empty() {
                    self.state.show_toast("Por favor escribe un nombre para el proyecto".to_string());
                } else {
                    self.state.create_new_project(&name);
                    self.state.new_project_dialog = false;
                }
            } else if close_dialog {
                self.state.new_project_dialog = false;
            }
        }
    }
}

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 750.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("Hypom"),
        ..Default::default()
    };

    eframe::run_native(
        "Hypom",
        native_options,
        Box::new(|cc| Ok(Box::new(HypomApp::new(cc)))),
    )
}
