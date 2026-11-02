use egui::{Color32, Stroke};

#[allow(dead_code)]
pub struct Theme {
    pub block_bg: Color32,
    pub block_header_bg: Color32,
    pub block_border: Stroke,
    pub row_hover_bg: Color32,
    pub row_alt_bg: Color32,
    pub accent: Color32,
    pub warning: Color32,
    pub comment_color: Color32,
    pub command_color: Color32,
}

#[allow(dead_code)]
impl Theme {
    pub fn dark() -> Self {
        Self {
            block_bg: Color32::from_rgb(28, 30, 36),
            block_header_bg: Color32::from_rgb(38, 42, 50),
            block_border: Stroke::new(1.0, Color32::from_rgb(55, 60, 72)),
            row_hover_bg: Color32::from_rgb(45, 50, 62),
            row_alt_bg: Color32::from_rgb(32, 35, 43),
            accent: Color32::from_rgb(79, 140, 255),
            warning: Color32::from_rgb(255, 193, 7),
            comment_color: Color32::from_rgb(108, 117, 125),
            command_color: Color32::from_rgb(130, 180, 255),
        }
    }

    pub fn light() -> Self {
        Self {
            block_bg: Color32::from_rgb(248, 249, 250),
            block_header_bg: Color32::from_rgb(233, 236, 239),
            block_border: Stroke::new(1.0, Color32::from_rgb(206, 212, 218)),
            row_hover_bg: Color32::from_rgb(222, 226, 230),
            row_alt_bg: Color32::from_rgb(241, 243, 245),
            accent: Color32::from_rgb(13, 110, 253),
            warning: Color32::from_rgb(217, 119, 6),
            comment_color: Color32::from_rgb(108, 117, 125),
            command_color: Color32::from_rgb(30, 64, 175),
        }
    }
}
