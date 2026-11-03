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

/// View-only filter: field contains text (case-insensitive)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterOpt {
    pub field: String,
    pub text: String,
}

/// Alert on dates within a window: field < Nd / Nw / Nm
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertOpt {
    pub field: String,
    pub days: i64,
}

/// Block options parsed from the header
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub order: Option<SortOrder>,
    pub copy_template: Option<String>,
    pub sep: Separator,
    pub mode: LayoutMode,
    pub view: ViewDefault,
    /// View-only filter (`filtro=campo~texto`)
    pub filter: Option<FilterOpt>,
    /// Highlight dates within window (`alerta=campo<7d`)
    pub alert: Option<AlertOpt>,
    /// Sum numeric column at footer (`total=campo`)
    pub total: Option<String>,
    /// Mark duplicate values (`unico=campo`)
    pub unique: Option<String>,
    /// Start collapsed
    pub collapsed: bool,
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
            filter: None,
            alert: None,
            total: None,
            unique: None,
            collapsed: false,
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
                "filtro" | "filter" => {
                    if let Some(f) = parse_filter(value) {
                        opts.filter = Some(f);
                    } else {
                        opts.unknown.push((key.clone(), value.clone()));
                    }
                }
                "alerta" | "alert" => {
                    if let Some(a) = parse_alert(value) {
                        opts.alert = Some(a);
                    } else {
                        opts.unknown.push((key.clone(), value.clone()));
                    }
                }
                "total" => {
                    opts.total = Some(value.trim().to_string());
                }
                "unico" | "unique" => {
                    opts.unique = Some(value.trim().to_string());
                }
                "colapsado" | "collapsed" => {
                    // Presence or true/false
                    let v = value.trim().to_lowercase();
                    opts.collapsed = v.is_empty() || v == "true" || v == "si" || v == "1";
                }
                _ => {
                    opts.unknown.push((key.clone(), value.clone()));
                }
            }
        }
        opts
    }

    /// Merge another options set on top (overrides non-default fields from `override_opts`)
    pub fn merge_override(&self, override_opts: &Options) -> Options {
        let mut merged = self.clone();
        if override_opts.order.is_some() {
            merged.order = override_opts.order.clone();
        }
        if override_opts.copy_template.is_some() {
            merged.copy_template = override_opts.copy_template.clone();
        }
        if override_opts.sep != Separator::Space {
            merged.sep = override_opts.sep.clone();
        }
        if override_opts.mode != LayoutMode::Auto {
            merged.mode = override_opts.mode.clone();
        }
        if override_opts.view != ViewDefault::Table {
            merged.view = override_opts.view.clone();
        }
        if override_opts.filter.is_some() {
            merged.filter = override_opts.filter.clone();
        }
        if override_opts.alert.is_some() {
            merged.alert = override_opts.alert.clone();
        }
        if override_opts.total.is_some() {
            merged.total = override_opts.total.clone();
        }
        if override_opts.unique.is_some() {
            merged.unique = override_opts.unique.clone();
        }
        if override_opts.collapsed {
            merged.collapsed = true;
        }
        merged.unknown.extend(override_opts.unknown.clone());
        merged
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

/// Parse `campo~texto`
fn parse_filter(value: &str) -> Option<FilterOpt> {
    let (field, text) = value.split_once('~')?;
    let field = field.trim();
    let text = text.trim();
    if field.is_empty() || text.is_empty() {
        return None;
    }
    Some(FilterOpt {
        field: field.to_string(),
        text: text.to_string(),
    })
}

/// Parse `campo<7d` / `campo<2w` / `campo<1m`
fn parse_alert(value: &str) -> Option<AlertOpt> {
    let (field, rest) = value.split_once('<')?;
    let field = field.trim();
    let rest = rest.trim().to_lowercase();
    if field.is_empty() || rest.is_empty() {
        return None;
    }

    let (num_str, unit) = if let Some(stripped) = rest.strip_suffix('d') {
        (stripped, 'd')
    } else if let Some(stripped) = rest.strip_suffix('w') {
        (stripped, 'w')
    } else if let Some(stripped) = rest.strip_suffix('m') {
        (stripped, 'm')
    } else {
        return None;
    };

    let n: i64 = num_str.trim().parse().ok()?;
    let days = match unit {
        'd' => n,
        'w' => n * 7,
        'm' => n * 30,
        _ => return None,
    };

    Some(AlertOpt {
        field: field.to_string(),
        days,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_filter_alert_total() {
        let pairs = vec![
            ("filtro".into(), "correo~gmail".into()),
            ("alerta".into(), "vence<7d".into()),
            ("total".into(), "monto".into()),
            ("unico".into(), "correo".into()),
            ("colapsado".into(), "".into()),
        ];
        let opts = Options::parse_from_pairs(&pairs);
        assert_eq!(opts.filter.as_ref().unwrap().field, "correo");
        assert_eq!(opts.filter.as_ref().unwrap().text, "gmail");
        assert_eq!(opts.alert.as_ref().unwrap().field, "vence");
        assert_eq!(opts.alert.as_ref().unwrap().days, 7);
        assert_eq!(opts.total.as_deref(), Some("monto"));
        assert_eq!(opts.unique.as_deref(), Some("correo"));
        assert!(opts.collapsed);
    }

    #[test]
    fn test_alert_weeks() {
        let a = parse_alert("fecha<2w").unwrap();
        assert_eq!(a.days, 14);
    }
}
