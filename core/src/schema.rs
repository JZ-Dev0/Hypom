use std::fmt;

/// Types of fields supported in blocks
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldType {
    Text,
    Number,
    Date,
    Secret,
    Url,
    Email,
    Check,
}

impl FieldType {
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "texto" | "text" => Some(Self::Text),
            "numero" | "number" | "num" => Some(Self::Number),
            "fecha" | "date" => Some(Self::Date),
            "secreto" | "secret" => Some(Self::Secret),
            "url" => Some(Self::Url),
            "email" | "correo" => Some(Self::Email),
            "check" | "checkbox" => Some(Self::Check),
            _ => None,
        }
    }
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text => write!(f, "texto"),
            Self::Number => write!(f, "numero"),
            Self::Date => write!(f, "fecha"),
            Self::Secret => write!(f, "secreto"),
            Self::Url => write!(f, "url"),
            Self::Email => write!(f, "email"),
            Self::Check => write!(f, "check"),
        }
    }
}

/// Names that auto-detect as secret type
const SECRET_NAMES: &[&str] = &[
    "pass", "password", "contra", "contraseña", "clave", "token", "secret", "pin",
];

/// A single field definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub ty: FieldType,
}

impl Field {
    pub fn new(name: impl Into<String>, ty: FieldType) -> Self {
        Self { name: name.into(), ty }
    }

    /// Create a field, auto-detecting secret type from name if no explicit type
    pub fn new_auto(name: impl Into<String>, explicit_ty: Option<FieldType>) -> Self {
        let name = name.into();
        let ty = explicit_ty.unwrap_or_else(|| {
            if SECRET_NAMES.iter().any(|s| s.eq_ignore_ascii_case(&name)) {
                FieldType::Secret
            } else {
                FieldType::Text
            }
        });
        Self { name, ty }
    }
}

/// Schema of a block — ordered list of fields
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    pub fields: Vec<Field>,
}

impl Schema {
    pub fn new(fields: Vec<Field>) -> Self {
        Self { fields }
    }

    pub fn empty() -> Self {
        Self { fields: Vec::new() }
    }

    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name))
    }

    /// Generate schema from inferred column count
    pub fn inferred(count: usize) -> Self {
        Self {
            fields: (1..=count)
                .map(|i| Field::new(format!("col{}", i), FieldType::Text))
                .collect(),
        }
    }
}
