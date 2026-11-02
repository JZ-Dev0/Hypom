use egui::{Color32, FontId, TextEdit, Ui};
use hypom_core::parse::TextSegment;

pub fn render_text_segment(ui: &mut Ui, segment: &mut TextSegment) -> bool {
    let mut changed = false;

    // Use a custom layouter for simple highlighting of @ commands and @// comments
    let mut layouter = |ui: &Ui, string: &str, wrap_width: f32| {
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = wrap_width;

        for line in string.split_inclusive('\n') {
            let trimmed = line.trim_start();
            let color = if trimmed.starts_with("@//") {
                Color32::from_rgb(120, 130, 140) // comment gray
            } else if trimmed.starts_with('@') {
                Color32::from_rgb(90, 160, 255) // command blue
            } else if trimmed.starts_with("\\@") {
                Color32::from_rgb(180, 140, 220) // escaped purple
            } else {
                ui.visuals().text_color()
            };

            let font_id = FontId::monospace(14.0);
            job.append(
                line,
                0.0,
                egui::text::TextFormat {
                    font_id,
                    color,
                    ..Default::default()
                },
            );
        }

        ui.fonts(|f| f.layout_job(job))
    };

    let response = ui.add(
        TextEdit::multiline(&mut segment.text)
            .desired_width(f32::INFINITY)
            .layouter(&mut layouter)
            .frame(false),
    );

    if response.changed() {
        changed = true;
    }

    changed
}

