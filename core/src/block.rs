use std::ops::Range;
use crate::schema::Schema;
use crate::options::Options;

/// Kind of block
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockKind {
    Tabla,
    Registro,
}

impl BlockKind {
    pub fn from_command(cmd: &str) -> Option<Self> {
        match cmd.to_lowercase().as_str() {
            "tabla" => Some(Self::Tabla),
            "registro" => Some(Self::Registro),
            _ => None,
        }
    }
}

/// How the block body is organized
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Layout {
    Row,
    Vertical,
}

/// How the block was closed
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Closing {
    /// Closed with @fin
    Fin,
    /// Closed because the next block started
    NextBlock,
    /// Closed because the file ended
    EndOfFile,
}

/// A row of data in a block
#[derive(Debug, Clone)]
pub struct Row {
    /// One value per field
    pub cells: Vec<String>,
    /// Source line range in the body (0-indexed within the block body)
    pub src_lines: Range<usize>,
}

/// An issue in a block (non-fatal)
#[derive(Debug, Clone)]
pub struct Issue {
    pub line: usize,
    pub message: String,
}

/// A fully parsed block
#[derive(Debug, Clone)]
pub struct ParsedBlock {
    pub kind: BlockKind,
    pub title: Option<String>,
    pub schema: Schema,
    pub options: Options,
    pub layout: Layout,
    pub rows: Vec<Row>,
    pub issues: Vec<Issue>,
    pub closed: Closing,
}
