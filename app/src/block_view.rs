use std::collections::HashSet;
use egui::{Color32, Frame, RichText, Sense, Stroke, TextEdit, Ui};
use egui_extras::{Column, TableBuilder};
use hypom_core::block::{BlockKind, Layout, ParsedBlock};
use hypom_core::dates::DateOrder;
use hypom_core::ops::{
    add_row, column_total, copy_row, duplicate_row_indices, edit_cell, is_alert_cell, is_checked,
    remove_row, sort_rows, toggle_check, visible_row_indices,
};
use hypom_core::options::{Separator, SortOrder};
use hypom_core::parse::{BlockSegment, ViewState};
use hypom_core::patterns::serialize_pares;
use hypom_core::schema::FieldType;
use hypom_core::serialize::serialize_row;
use crate::clipboard::ClipboardManager;
use crate::icons;

pub struct BlockUiState {
    pub editing_cell: Option<(usize, usize)>, // (row_idx, col_idx)
    pub edit_buffer: String,
    pub focus_edit: bool,
    pub revealed_secrets: HashSet<(usize, usize)>,
    pub manually_collapsed: Option<bool>,
}

impl Default for BlockUiState {
    fn default() -> Self {
        Self {
            editing_cell: None,
            edit_buffer: String::new(),
            focus_edit: false,
            revealed_secrets: HashSet::new(),
            manually_collapsed: None,
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
    // Fill the editor width so the raw editor grows/shrinks with the window
    ui.set_min_width(ui.available_width());

    Frame::group(ui.style())
        .fill(Color32::from_rgb(24, 26, 32))
        .stroke(Stroke::new(1.0, Color32::from_rgb(60, 65, 78)))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            let mut switch_to_table = false;
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Modo Crudo (Texto)").strong().color(Color32::from_rgb(100, 160, 255)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(format!("{} Ver como tabla", icons::TABLE))
                        .clicked()
                    {
                        switch_to_table = true;
                    }
                });
            });

            ui.add_space(4.0);

            let resp = ui.add(
                TextEdit::multiline(&mut segment.raw)
                    .desired_width(f32::INFINITY)
                    .desired_rows(6)
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

fn kind_label(kind: &BlockKind) -> String {
    match kind {
        BlockKind::Tabla => "tabla".into(),
        BlockKind::Registro => "registro".into(),
        BlockKind::Lista => "lista".into(),
        BlockKind::Custom(name) => name.clone(),
        BlockKind::Pares => "pares".into(),
    }
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
    let collapsed = ui_state
        .manually_collapsed
        .unwrap_or(segment.parsed.options.collapsed);

    // Stretch block frame to the current panel width
    ui.set_min_width(ui.available_width());

    Frame::group(ui.style())
        .fill(Color32::from_rgb(26, 29, 36))
        .stroke(Stroke::new(
            1.0,
            if matches!(segment.parsed.kind, BlockKind::Pares) {
                Color32::from_rgb(70, 110, 90)
            } else {
                Color32::from_rgb(50, 56, 68)
            },
        ))
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            let block = &mut segment.parsed;

            // Header bar — wraps on narrow windows
            ui.horizontal_wrapped(|ui| {
                let kind = kind_label(&block.kind);
                let prefix = if matches!(block.kind, BlockKind::Pares) {
                    icons::APPROXIMATE_EQUALS
                } else {
                    "@"
                };

                let header_title = if let Some(title) = &block.title {
                    format!("{}{} · \"{}\"", prefix, kind, title)
                } else {
                    format!("{}{}", prefix, kind)
                };

                ui.label(
                    RichText::new(header_title)
                        .strong()
                        .color(Color32::from_rgb(130, 180, 255)),
                );

                if !block.issues.is_empty() {
                    let issue_text =
                        format!("{} {} avisos", icons::WARNING, block.issues.len());
                    ui.label(RichText::new(issue_text).color(Color32::from_rgb(255, 193, 7)));
                }

                if let Some(f) = &block.options.filter {
                    ui.label(
                        RichText::new(format!(
                            "{} {}:{}",
                            icons::FUNNEL,
                            f.field,
                            f.text
                        ))
                        .small()
                        .color(Color32::from_rgb(160, 200, 140)),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let collapse_label = if collapsed {
                        icons::CARET_RIGHT
                    } else {
                        icons::CARET_DOWN
                    };
                    if ui
                        .button(collapse_label)
                        .on_hover_text("Colapsar / expandir")
                        .clicked()
                    {
                        ui_state.manually_collapsed = Some(!collapsed);
                    }

                    if !matches!(block.kind, BlockKind::Pares) {
                        if ui
                            .button(format!("{} Texto", icons::CODE))
                            .on_hover_text("Cambiar a vista de texto crudo")
                            .clicked()
                        {
                            switch_to_raw = true;
                        }
                    }

                    if !collapsed
                        && !matches!(block.kind, BlockKind::Pares)
                        && ui
                            .button(format!("{} Fila", icons::ROWS_PLUS_BOTTOM))
                            .on_hover_text("Agregar nueva fila al bloque")
                            .clicked()
                    {
                        add_row(block);
                        let new_row_idx = block.rows.len() - 1;
                        ui_state.editing_cell = Some((new_row_idx, 0));
                        ui_state.edit_buffer = String::new();
                        ui_state.focus_edit = true;
                        *modified = true;
                    }
                });
            });

            if collapsed {
                ui.label(
                    RichText::new(format!("{} filas", block.rows.len()))
                        .weak()
                        .italics(),
                );
                return;
            }

            ui.add_space(6.0);

            let num_fields = block.schema.fields.len();
            if num_fields == 0 {
                ui.label(RichText::new("Bloque sin columnas definidas").italics().weak());
                return;
            }

            let visible_rows = visible_row_indices(block);
            let dup_rows = duplicate_row_indices(block);

            // Responsive columns: share remaining width equally and reflow on resize.
            // Avoid Column::initial + resizable — egui freezes those widths across frames.
            let panel_w = ui.available_width();
            let narrow = panel_w < 520.0;
            let actions_w = if narrow { 52.0 } else { 68.0 };
            let min_col = if narrow { 56.0 } else { 72.0 };

            let builder = TableBuilder::new(ui)
                .id_salt("block_table")
                .striped(true)
                .vscroll(false) // outer editor ScrollArea handles vertical scroll
                .auto_shrink([false, true]) // fill width, shrink height to content
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(actions_w))
                .columns(
                    Column::remainder()
                        .at_least(min_col)
                        .clip(true)
                        .resizable(false),
                    num_fields,
                );

            let mut sort_field_clicked: Option<usize> = None;
            let mut row_to_delete: Option<usize> = None;
            let mut cell_edited: Option<(usize, usize, String)> = None;
            let mut check_toggled: Option<(usize, usize)> = None;

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

                            if let Some(order) = &block.options.order {
                                match order {
                                    SortOrder::Asc(f) if f.eq_ignore_ascii_case(&field.name) => {
                                        text.push(' ');
                                        text.push_str(icons::ARROW_UP);
                                    }
                                    SortOrder::Desc(f) if f.eq_ignore_ascii_case(&field.name) => {
                                        text.push(' ');
                                        text.push_str(icons::ARROW_DOWN);
                                    }
                                    SortOrder::Closest(f) if f.eq_ignore_ascii_case(&field.name) => {
                                        text.push(' ');
                                        text.push_str(icons::CLOCK);
                                    }
                                    _ => {}
                                }
                            }

                            let resp = ui
                                .button(RichText::new(text).strong())
                                .on_hover_text("Clic para ordenar");
                            if resp.clicked() {
                                sort_field_clicked = Some(col_idx);
                            }
                        });
                    }
                })
                .body(|mut body| {
                    for &row_idx in &visible_rows {
                        let is_dup = dup_rows.contains(&row_idx);
                        body.row(24.0, |mut row| {
                            row.col(|ui| {
                                ui.push_id(row_idx, |ui| {
                                    ui.horizontal(|ui| {
                                        if ui
                                            .button(icons::CLIPBOARD_TEXT)
                                            .on_hover_text("Copiar registro completo")
                                            .clicked()
                                        {
                                            if let Some(text) = copy_row(block, row_idx) {
                                                clipboard.copy_text(&text);
                                                *toast_msg =
                                                    Some("Registro copiado al portapapeles".to_string());
                                            }
                                        }

                                        if !matches!(block.kind, BlockKind::Pares)
                                            && ui
                                                .button(icons::TRASH)
                                                .on_hover_text("Eliminar fila")
                                                .clicked()
                                        {
                                            row_to_delete = Some(row_idx);
                                        }
                                    });
                                });
                            });

                            for col_idx in 0..num_fields {
                                row.col(|ui| {
                                    ui.push_id(col_idx, |ui| {
                                        let field_type = block
                                            .schema
                                            .fields
                                            .get(col_idx)
                                            .map(|f| &f.ty)
                                            .unwrap_or(&FieldType::Text);
                                        let cell_val = block
                                            .rows
                                            .get(row_idx)
                                            .and_then(|r| r.cells.get(col_idx))
                                            .cloned()
                                            .unwrap_or_default();

                                        let is_editing =
                                            ui_state.editing_cell == Some((row_idx, col_idx));
                                        let alert =
                                            is_alert_cell(block, row_idx, col_idx, date_order);

                                        if *field_type == FieldType::Check && !is_editing {
                                            let mut checked = is_checked(&cell_val);
                                            let resp = ui.checkbox(&mut checked, "");
                                            if resp.changed() {
                                                check_toggled = Some((row_idx, col_idx));
                                            }
                                            return;
                                        }

                                        if is_editing {
                                            let text_edit =
                                                TextEdit::singleline(&mut ui_state.edit_buffer)
                                                    .desired_width(f32::INFINITY);
                                            let resp = ui.add(text_edit);
                                            if ui_state.focus_edit {
                                                resp.request_focus();
                                                ui_state.focus_edit = false;
                                            }

                                            let enter_pressed =
                                                ui.input(|i| i.key_pressed(egui::Key::Enter));
                                            let tab_pressed =
                                                ui.input(|i| i.key_pressed(egui::Key::Tab));
                                            let escape_pressed =
                                                ui.input(|i| i.key_pressed(egui::Key::Escape));

                                            if escape_pressed {
                                                ui_state.editing_cell = None;
                                            } else if enter_pressed || tab_pressed {
                                                cell_edited = Some((
                                                    row_idx,
                                                    col_idx,
                                                    ui_state.edit_buffer.clone(),
                                                ));
                                                if col_idx + 1 < num_fields {
                                                    let next_val = block
                                                        .rows
                                                        .get(row_idx)
                                                        .and_then(|r| r.cells.get(col_idx + 1))
                                                        .cloned()
                                                        .unwrap_or_default();
                                                    ui_state.editing_cell =
                                                        Some((row_idx, col_idx + 1));
                                                    ui_state.edit_buffer = next_val;
                                                    ui_state.focus_edit = true;
                                                } else {
                                                    ui_state.editing_cell = None;
                                                }
                                            } else if resp.lost_focus() {
                                                cell_edited = Some((
                                                    row_idx,
                                                    col_idx,
                                                    ui_state.edit_buffer.clone(),
                                                ));
                                                ui_state.editing_cell = None;
                                            }
                                        } else {
                                            let is_secret = *field_type == FieldType::Secret;
                                            let is_revealed = ui_state
                                                .revealed_secrets
                                                .contains(&(row_idx, col_idx));
                                            let is_empty = cell_val.trim().is_empty();

                                            let display_str = if is_secret && !is_revealed {
                                                "••••••••".to_string()
                                            } else if is_empty {
                                                "—".to_string()
                                            } else {
                                                cell_val.clone()
                                            };

                                            let text_color = if alert {
                                                Color32::from_rgb(255, 160, 60)
                                            } else if is_dup {
                                                Color32::from_rgb(255, 140, 140)
                                            } else if is_secret {
                                                Color32::from_rgb(200, 170, 255)
                                            } else if is_empty {
                                                Color32::from_rgb(110, 115, 130)
                                            } else {
                                                ui.visuals().text_color()
                                            };

                                            let mut rt =
                                                RichText::new(&display_str).color(text_color);
                                            if is_empty {
                                                rt = rt.italics();
                                            }
                                            if alert {
                                                rt = rt.strong();
                                            }

                                            let label_resp =
                                                ui.add(egui::Label::new(rt).sense(Sense::click()));

                                            if label_resp.double_clicked()
                                                || (is_empty && label_resp.clicked())
                                            {
                                                ui_state.editing_cell = Some((row_idx, col_idx));
                                                ui_state.edit_buffer = cell_val.clone();
                                                ui_state.focus_edit = true;
                                            } else if label_resp.clicked() {
                                                if is_secret {
                                                    if is_revealed {
                                                        ui_state
                                                            .revealed_secrets
                                                            .remove(&(row_idx, col_idx));
                                                    } else {
                                                        ui_state
                                                            .revealed_secrets
                                                            .insert((row_idx, col_idx));
                                                    }
                                                    clipboard
                                                        .copy_secret(&cell_val, clipboard_clear_secs);
                                                    *toast_msg = Some(format!(
                                                        "Secreto copiado (se limpiará en {}s)",
                                                        clipboard_clear_secs
                                                    ));
                                                } else {
                                                    clipboard.copy_text(&cell_val);
                                                    *toast_msg =
                                                        Some(format!("Copiado: {}", cell_val));
                                                }
                                            }

                                            label_resp.on_hover_text(if alert {
                                                "Fecha dentro de la ventana de alerta"
                                            } else if is_empty {
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

            // Footer: total
            if let Some((field, sum)) = column_total(block) {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!(
                        "{} {} = {}",
                        icons::SIGMA,
                        field,
                        format_number(sum)
                    ))
                    .strong()
                    .color(Color32::from_rgb(140, 210, 160)),
                );
            }

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

            if let Some(idx) = row_to_delete {
                remove_row(block, idx);
                *modified = true;
            }

            if let Some((r, c, new_val)) = cell_edited {
                edit_cell(block, r, c, new_val);
                *modified = true;
            }

            if let Some((r, c)) = check_toggled {
                let current = block
                    .rows
                    .get(r)
                    .and_then(|row| row.cells.get(c))
                    .cloned()
                    .unwrap_or_default();
                edit_cell(block, r, c, toggle_check(&current));
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

fn format_number(n: f64) -> String {
    if (n.fract()).abs() < f64::EPSILON {
        format!("{:.0}", n)
    } else {
        format!("{:.2}", n)
    }
}

pub fn sync_raw_from_block(block: &ParsedBlock) -> String {
    if matches!(block.kind, BlockKind::Pares) || matches!(block.layout, Layout::KeyValue) {
        return serialize_pares(block);
    }

    let mut lines = Vec::new();

    let kind_str = match &block.kind {
        BlockKind::Tabla => "tabla".to_string(),
        BlockKind::Registro => "registro".to_string(),
        BlockKind::Lista => "lista".to_string(),
        BlockKind::Custom(name) => name.clone(),
        BlockKind::Pares => "pares".to_string(),
    };

    let mut header_line = format!("@{}", kind_str);
    if let Some(t) = &block.title {
        header_line.push_str(&format!(" \"{}\"", t));
    }

    // Lista with default schema: omit fields in header for cleaner text
    let omit_lista_fields = matches!(block.kind, BlockKind::Lista)
        && block.schema.fields.len() == 2
        && block.schema.fields[0].name == "hecho"
        && block.schema.fields[1].name == "item";

    if !block.schema.fields.is_empty()
        && !omit_lista_fields
        && !matches!(block.kind, BlockKind::Custom(_))
    {
        let fields_str: Vec<String> = block
            .schema
            .fields
            .iter()
            .map(|f| {
                if f.ty == FieldType::Text {
                    f.name.clone()
                } else {
                    format!("{}:{}", f.name, f.ty)
                }
            })
            .collect();
        header_line.push_str(&format!(" {}", fields_str.join(", ")));
    }

    // For Custom, schema comes from @def — don't restate fields unless overridden
    // (title-only usage is fine)

    let mut opts_parts = Vec::new();
    if let Some(order) = &block.options.order {
        match order {
            SortOrder::Asc(f) => opts_parts.push(format!("orden={}", f)),
            SortOrder::Desc(f) => opts_parts.push(format!("orden={} desc", f)),
            SortOrder::Closest(f) => opts_parts.push(format!("orden={} cercana", f)),
        }
    }
    // For @def custom types, copy_template usually lives on the @def line — don't rewrite it here
    if let Some(tpl) = &block.options.copy_template {
        if !matches!(block.kind, BlockKind::Custom(_)) {
            opts_parts.push(format!("copiar={}", tpl));
        }
    }
    if block.options.sep != Separator::Space {
        let sep_name = match block.options.sep {
            Separator::Tab => "tab",
            Separator::Pipe => "pipe",
            Separator::Comma => "coma",
            Separator::Semicolon => "semicolon",
            Separator::Space => "espacio",
        };
        opts_parts.push(format!("sep={}", sep_name));
    }
    if matches!(block.options.mode, hypom_core::options::LayoutMode::Vertical) {
        opts_parts.push("modo=vertical".into());
    } else if matches!(block.options.mode, hypom_core::options::LayoutMode::Row) {
        opts_parts.push("modo=fila".into());
    }
    if let Some(f) = &block.options.filter {
        opts_parts.push(format!("filtro={}~{}", f.field, f.text));
    }
    if let Some(a) = &block.options.alert {
        opts_parts.push(format!("alerta={}<{}d", a.field, a.days));
    }
    if let Some(t) = &block.options.total {
        opts_parts.push(format!("total={}", t));
    }
    if let Some(u) = &block.options.unique {
        opts_parts.push(format!("unico={}", u));
    }
    if block.options.collapsed {
        opts_parts.push("colapsado".into());
    }

    if !opts_parts.is_empty() {
        header_line.push_str(&format!(" | {}", opts_parts.join(" | ")));
    }

    lines.push(header_line);

    for row in &block.rows {
        match block.kind {
            BlockKind::Lista => {
                let check = row.cells.first().map(|s| s.as_str()).unwrap_or("[ ]");
                let item = row.cells.get(1).map(|s| s.as_str()).unwrap_or("");
                let mark = if is_checked(check) { "[x]" } else { "[ ]" };
                lines.push(format!("{} {}", mark, item));
            }
            _ => match block.layout {
                Layout::Row => {
                    lines.push(serialize_row(row, &block.options.sep));
                }
                Layout::Vertical | Layout::KeyValue => {
                    for cell in &row.cells {
                        lines.push(cell.clone());
                    }
                    lines.push(String::new());
                }
            },
        }
    }

    lines.push("@fin".to_string());
    lines.join("\n")
}
