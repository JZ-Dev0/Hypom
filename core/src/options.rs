/// Sort direction
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortOrder {
    Asc(String),       // field name, ascending
    Desc(String),      // field name, descending
    Closest(String),   // field name, closest date first
}

/// Separator for row mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Separator {
    Space,
    Tab,
    Pipe,
    Comma,
    Semicolon,
}

impl Separator {
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "espacio" | "space" => Some(Self::Space),
            "tab" | "tabulador" => Some(Self::Tab),
            "\\|" | "pipe" => Some(Self::Pipe),
            "," | "coma" | "comma" => Some(Self::Comma),
            ";" | "punto y coma" | "semicolon" => Some(Self::Semicolon),
            _ => None,
        }
    }

    pub fn char(&self) -> Option<char> {
        match self {
            Self::Space => None, // special handling
            Self::Tab => Some('\t'),
            Self::Pipe => Some('|'),
            Self::Comma => Some(','),
            Self::Semicolon => Some(';'),
        }
    }
}

/// Layout mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutMode {
    Auto,
    Row,
    Vertical,
}

/// Default view state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewDefault {
    Table,
    Raw,
}

/// Block options parsed from the header
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub order: Option<SortOrder>,
    pub copy_template: Option<String>,
    pub sep: Separator,
    pub mode: LayoutMode,
    pub view: ViewDefault,
    pub unknown: Vec<(String, String)>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            order: None,
            copy_template: None,
            sep: Separator::Space,
            mode: LayoutMode::Auto,
            view: ViewDefault::Table,
            unknown: Vec::new(),
        }
    }
}

impl Options {
    pub fn parse_from_pairs(pairs: &[(String, String)]) -> Self {
        let mut opts = Self::default();
        for (key, value) in pairs {
            match key.to_lowercase().as_str() {
                "orden" | "order" => {
                    opts.order = Some(parse_sort_order(value));
                }
                "copiar" | "copy" => {
                    opts.copy_template = Some(value.clone());
                }
                "sep" => {
                    if let Some(s) = Separator::from_str_opt(value) {
                        opts.sep = s;
                    } else {
                        opts.unknown.push((key.clone(), value.clone()));
                    }
                }
                "modo" | "mode" => {
                    match value.to_lowercase().as_str() {
                        "auto" => opts.mode = LayoutMode::Auto,
                        "fila" | "row" => opts.mode = LayoutMode::Row,
                        "vertical" => opts.mode = LayoutMode::Vertical,
                        _ => opts.unknown.push((key.clone(), value.clone())),
                    }
                }
                "vista" | "view" => {
                    match value.to_lowercase().as_str() {
                        "tabla" | "table" => opts.view = ViewDefault::Table,
                        "crudo" | "raw" => opts.view = ViewDefault::Raw,
                        _ => opts.unknown.push((key.clone(), value.clone())),
                    }
                }
                _ => {
                    opts.unknown.push((key.clone(), value.clone()));
                }
            }
        }
        opts
    }
}

fn parse_sort_order(value: &str) -> SortOrder {
    let parts: Vec<&str> = value.trim().splitn(2, ' ').collect();
    let field = parts[0].to_string();
    if parts.len() > 1 {
        match parts[1].to_lowercase().as_str() {
            "desc" => SortOrder::Desc(field),
            "cercana" | "closest" => SortOrder::Closest(field),
            _ => SortOrder::Asc(field),
        }
    } else {
        SortOrder::Asc(field)
    }
}
