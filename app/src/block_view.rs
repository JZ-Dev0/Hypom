use std::collections::HashSet;
use egui::{Color32, Frame, RichText, Sense, Stroke, TextEdit, Ui};
use egui_extras::{Column, TableBuilder};
use hypom_core::block::{BlockKind, Layout, ParsedBlock};
use hypom_core::dates::DateOrder;
use hypom_core::ops::{add_row, copy_row, edit_cell, remove_row, sort_rows};
use hypom_core::options::{Separator, SortOrder};
use hypom_core::parse::{BlockSegment, ViewState};
use hypom_core::schema::FieldType;
use hypom_core::serialize::serialize_row;
use crate::clipboard::ClipboardManager;

pub struct BlockUiState {
    pub editing_cell: Option<(usize, usize)>, // (row_idx, col_idx)
    pub edit_buffer: String,
    pub focus_edit: bool,
    pub revealed_secrets: HashSet<(usize, usize)>,
}

impl Default for BlockUiState {
    fn default() -> Self {
        Self {
            editing_cell: None,
            edit_buffer: String::new(),
            focus_edit: false,
            revealed_secrets: HashSet::new(),
        }
    }
}

pub fn render_block_segment(
    ui: &mut Ui,
    segment: &mut BlockSegment,
    ui_state: &mut BlockUiState,
    clipboard: &mut ClipboardManager,
    clipboard_clear_secs: u64,
    date_order: &DateOrder,
    toast_msg: &mut Option<String>,
) -> bool {
    let mut modified = false;

    match segment.state {
        ViewState::Raw => {
            render_raw_view(ui, segment, &mut modified);
        }
        ViewState::Table => {
            render_table_view(
                ui,
                segment,
                ui_state,
                clipboard,
                clipboard_clear_secs,
                date_order,
                toast_msg,
                &mut modified,
            );
        }
    }

    modified
}

fn render_raw_view(ui: &mut Ui, segment: &mut BlockSegment, modified: &mut bool) {
    Frame::group(ui.style())
        .fill(Color32::from_rgb(24, 26, 32))
        .stroke(Stroke::new(1.0, Color32::from_rgb(60, 65, 78)))
        .show(ui, |ui| {
            let mut switch_to_table = false;
            ui.horizontal(|ui| {
                ui.label(RichText::new("Modo Crudo (Texto)").strong().color(Color32::from_rgb(100, 160, 255)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✓ Ver como tabla").clicked() {
                        switch_to_table = true;
                    }
                });
            });

            ui.add_space(4.0);

            let resp = ui.add(
                TextEdit::multiline(&mut segment.raw)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace)
                    .code_editor(),
            );

            if resp.changed() {
                *modified = true;
            }

            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                switch_to_table = true;
            }

            if switch_to_table {
                let parsed_segments = hypom_core::parse::parse_note(&segment.raw);
                for seg in parsed_segments {
                    if let hypom_core::parse::Segment::Block(b) = seg {
                        segment.parsed = b.parsed;
                        break;
                    }
                }
                segment.state = ViewState::Table;
                *modified = true;
            }
        });
}

fn render_table_view(
    ui: &mut Ui,
    segment: &mut BlockSegment,
    ui_state: &mut BlockUiState,
    clipboard: &mut ClipboardManager,
    clipboard_clear_secs: u64,
    date_order: &DateOrder,
    toast_msg: &mut Option<String>,
    modified: &mut bool,
) {
    let mut switch_to_raw = false;

    Frame::group(ui.style())
        .fill(Color32::from_rgb(26, 29, 36))
        .stroke(Stroke::new(1.0, Color32::from_rgb(50, 56, 68)))
        .inner_margin(8.0)
        .show(ui, |ui| {
            let block = &mut segment.parsed;

            // Header bar
            ui.horizontal(|ui| {
                let kind_label = match block.kind {
                    BlockKind::Tabla => "tabla",
                    BlockKind::Registro => "registro",
                };

                let header_title = if let Some(title) = &block.title {
                    format!("@{} · \"{}\"", kind_label, title)
                } else {
                    format!("@{}", kind_label)
                };

                ui.label(RichText::new(header_title).strong().color(Color32::from_rgb(130, 180, 255)));

                if !block.issues.is_empty() {
                    let issue_text = format!("⚠ {} avisos", block.issues.len());
                    ui.label(RichText::new(issue_text).color(Color32::from_rgb(255, 193, 7)));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("</> Texto").on_hover_text("Cambiar a vista de texto crudo").clicked() {
                        switch_to_raw = true;
                    }

                    if ui.button("+ Fila").on_hover_text("Agregar nueva fila al bloque").clicked() {
                        add_row(block);
                        let new_row_idx = block.rows.len() - 1;
                        ui_state.editing_cell = Some((new_row_idx, 0));
                        ui_state.edit_buffer = String::new();
                        ui_state.focus_edit = true;
                        *modified = true;
                    }
                });
            });

            ui.add_space(6.0);

            let num_fields = block.schema.fields.len();
            if num_fields == 0 {
                ui.label(RichText::new("Bloque sin columnas definidas").italics().weak());
                return;
            }

            let available_width = ui.available_width() - 80.0;
            let col_width = (available_width / num_fields as f32).max(80.0);

            // Table builder
            let mut builder = TableBuilder::new(ui)
                .id_salt("block_table")
                .striped(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(65.0)); // Action buttons (📋, 🗑)

            for _ in 0..num_fields {
                builder = builder.column(Column::initial(col_width).resizable(true).clip(true));
            }

            let mut sort_field_clicked: Option<usize> = None;
            let mut row_to_delete: Option<usize> = None;
            let mut cell_edited: Option<(usize, usize, String)> = None;

            builder
                .header(24.0, |mut header| {
                    header.col(|ui| {
                        ui.label(RichText::new("Acciones").weak().size(11.0));
                    });

                    for (col_idx, field) in block.schema.fields.iter().enumerate() {
                        header.col(|ui| {
                            let mut text = field.name.clone();
                            if field.ty != FieldType::Text {
                                text = format!("{}:{}", text, field.ty);
                            }

                            // Show sort indicator if sorted by this field
                            if let Some(order) = &block.options.order {
                                match order {
                                    SortOrder::Asc(f) if f.eq_ignore_ascii_case(&field.name) => {
                                        text.push_str(" ▲");
                                    }
                                    SortOrder::Desc(f) if f.eq_ignore_ascii_case(&field.name) => {
                                        text.push_str(" ▼");
                                    }
                                    SortOrder::Closest(f) if f.eq_ignore_ascii_case(&field.name) => {
                                        text.push_str(" ◷");
                                    }
                                    _ => {}
                                }
                            }

                            let resp = ui.button(RichText::new(text).strong())
                                .on_hover_text("Clic para ordenar");
                            if resp.clicked() {
                                sort_field_clicked = Some(col_idx);
                            }
                        });
                    }
                })
                .body(|mut body| {
                    let row_count = block.rows.len();
                    for row_idx in 0..row_count {
                        body.row(24.0, |mut row| {
                            // Column 0: Row actions (Copy row, Delete row)
                            row.col(|ui| {
                                ui.push_id(row_idx, |ui| {
                                    ui.horizontal(|ui| {
                                        if ui.button("📋").on_hover_text("Copiar registro completo").clicked() {
                                            if let Some(text) = copy_row(block, row_idx) {
                                                clipboard.copy_text(&text);
                                                *toast_msg = Some("Registro copiado al portapapeles".to_string());
                                            }
                                        }

                                        if ui.button("🗑").on_hover_text("Eliminar fila").clicked() {
                                            row_to_delete = Some(row_idx);
                                        }
                                    });
                                });
                            });

                            // Data columns
                            for col_idx in 0..num_fields {
                                row.col(|ui| {
                                    ui.push_id(col_idx, |ui| {
                                        let field_type = block.schema.fields.get(col_idx).map(|f| &f.ty).unwrap_or(&FieldType::Text);
                                        let cell_val = block.rows.get(row_idx)
                                            .and_then(|r| r.cells.get(col_idx))
                                            .cloned()
                                            .unwrap_or_default();

                                        let is_editing = ui_state.editing_cell == Some((row_idx, col_idx));

                                        if is_editing {
                                            let text_edit = TextEdit::singleline(&mut ui_state.edit_buffer)
                                                .desired_width(f32::INFINITY);
                                            let resp = ui.add(text_edit);
                                            if ui_state.focus_edit {
                                                resp.request_focus();
                                                ui_state.focus_edit = false;
                                            }

                                            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                                            let tab_pressed = ui.input(|i| i.key_pressed(egui::Key::Tab));
                                            let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));

                                            if escape_pressed {
                                                ui_state.editing_cell = None;
                                            } else if enter_pressed || tab_pressed {
                                                cell_edited = Some((row_idx, col_idx, ui_state.edit_buffer.clone()));
                                                // Advance to next column if available
                                                if col_idx + 1 < num_fields {
                                                    let next_val = block.rows.get(row_idx)
                                                        .and_then(|r| r.cells.get(col_idx + 1))
                                                        .cloned()
                                                        .unwrap_or_default();
                                                    ui_state.editing_cell = Some((row_idx, col_idx + 1));
                                                    ui_state.edit_buffer = next_val;
                                                    ui_state.focus_edit = true;
                                                } else {
                                                    ui_state.editing_cell = None;
                                                }
                                            } else if resp.lost_focus() {
                                                cell_edited = Some((row_idx, col_idx, ui_state.edit_buffer.clone()));
                                                ui_state.editing_cell = None;
                                            }
                                        } else {
                                            let is_secret = *field_type == FieldType::Secret;
                                            let is_revealed = ui_state.revealed_secrets.contains(&(row_idx, col_idx));
                                            let is_empty = cell_val.trim().is_empty();

                                            let display_str = if is_secret && !is_revealed {
                                                "••••••••".to_string()
                                            } else if is_empty {
                                                "—".to_string()
                                            } else {
                                                cell_val.clone()
                                            };

                                            let text_color = if is_secret {
                                                Color32::from_rgb(200, 170, 255)
                                            } else if is_empty {
                                                Color32::from_rgb(110, 115, 130)
                                            } else {
                                                ui.visuals().text_color()
                                            };

                                            let mut rt = RichText::new(&display_str).color(text_color);
                                            if is_empty {
                                                rt = rt.italics();
                                            }

                                            let label_resp = ui.add(
                                                egui::Label::new(rt).sense(Sense::click()),
                                            );

                                            if label_resp.double_clicked() || (is_empty && label_resp.clicked()) {
                                                ui_state.editing_cell = Some((row_idx, col_idx));
                                                ui_state.edit_buffer = cell_val.clone();
                                                ui_state.focus_edit = true;
                                            } else if label_resp.clicked() {
                                                if is_secret {
                                                    if is_revealed {
                                                        ui_state.revealed_secrets.remove(&(row_idx, col_idx));
                                                    } else {
                                                        ui_state.revealed_secrets.insert((row_idx, col_idx));
                                                    }
                                                    clipboard.copy_secret(&cell_val, clipboard_clear_secs);
                                                    *toast_msg = Some(format!("Secreto copiado (se limpiará en {}s)", clipboard_clear_secs));
                                                } else {
                                                    clipboard.copy_text(&cell_val);
                                                    *toast_msg = Some(format!("Copiado: {}", cell_val));
                                                }
                                            }

                                            label_resp.on_hover_text(if is_empty {
                                                "Clic para editar campo vacío"
                                            } else {
                                                "Clic: copiar | Doble clic: editar"
                                            });
                                        }
                                    });
                                });
                            }
                        });
                    }
                });

            // Handle sort toggle
            if let Some(col_idx) = sort_field_clicked {
                let field_name = block.schema.fields[col_idx].name.clone();
                let new_order = match &block.options.order {
                    Some(SortOrder::Asc(f)) if f.eq_ignore_ascii_case(&field_name) => {
                        SortOrder::Desc(field_name)
                    }
                    Some(SortOrder::Desc(f)) if f.eq_ignore_ascii_case(&field_name) => {
                        if block.schema.fields[col_idx].ty == FieldType::Date {
                            SortOrder::Closest(field_name)
                        } else {
                            SortOrder::Asc(field_name)
                        }
                    }
                    _ => SortOrder::Asc(field_name),
                };
                block.options.order = Some(new_order);
                sort_rows(block, date_order);
                *modified = true;
            }

            // Handle row deletion
            if let Some(idx) = row_to_delete {
                remove_row(block, idx);
                *modified = true;
            }

            // Handle cell edit
            if let Some((r, c, new_val)) = cell_edited {
                edit_cell(block, r, c, new_val);
                *modified = true;
            }
        });

    if switch_to_raw {
        segment.state = ViewState::Raw;
    }

    if *modified {
        segment.raw = sync_raw_from_block(&segment.parsed);
    }
}

pub fn sync_raw_from_block(block: &ParsedBlock) -> String {
    let mut lines = Vec::new();

    // Reconstruct header line
    let kind_str = match block.kind {
        BlockKind::Tabla => "tabla",
        BlockKind::Registro => "registro",
    };

    let mut header_line = format!("@{}", kind_str);
    if let Some(t) = &block.title {
        header_line.push_str(&format!(" \"{}\"", t));
    }

    if !block.schema.fields.is_empty() {
        let fields_str: Vec<String> = block.schema.fields.iter().map(|f| {
            if f.ty == FieldType::Text {
                f.name.clone()
            } else {
                format!("{}:{}", f.name, f.ty)
            }
        }).collect();
        header_line.push_str(&format!(" {}", fields_str.join(", ")));
    }

    // Options
    let mut opts_parts = Vec::new();
    if let Some(order) = &block.options.order {
        match order {
            SortOrder::Asc(f) => opts_parts.push(format!("orden={}", f)),
            SortOrder::Desc(f) => opts_parts.push(format!("orden={} desc", f)),
            SortOrder::Closest(f) => opts_parts.push(format!("orden={} cercana", f)),
        }
    }
    if let Some(tpl) = &block.options.copy_template {
        opts_parts.push(format!("copiar={}", tpl));
    }
    if block.options.sep != Separator::Space {
        opts_parts.push(format!("sep={:?}", block.options.sep).to_lowercase());
    }

    if !opts_parts.is_empty() {
        header_line.push_str(&format!(" | {}", opts_parts.join(" | ")));
    }

    lines.push(header_line);

    // Body rows
    for row in &block.rows {
        match block.layout {
            Layout::Row => {
                lines.push(serialize_row(row, &block.options.sep));
            }
            Layout::Vertical => {
                for cell in &row.cells {
                    lines.push(cell.clone());
                }
                lines.push(String::new()); // blank line separating records
            }
        }
    }

    lines.push("@fin".to_string());
    lines.join("\n")
}
