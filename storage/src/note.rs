use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use hypom_core::parse::{detect_eol, parse_note, Eol, Segment};
use hypom_core::serialize::serialize_segments;

#[derive(Debug, Clone)]
pub struct LoadedNote {
    pub path: PathBuf,
    pub eol: Eol,
    pub segments: Vec<Segment>,
    pub dirty: bool,
    pub disk_mtime: SystemTime,
}

impl LoadedNote {
    pub fn title(&self) -> String {
        self.path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Sin título".to_string())
    }

    pub fn full_text(&self) -> String {
        serialize_segments(&self.segments, self.eol.as_str())
    }

    pub fn reparse_all(&mut self) {
        let full = self.full_text();
        self.segments = parse_note(&full);
    }
}

pub fn load_note(path: &Path) -> Result<LoadedNote, std::io::Error> {
    let bytes = fs::read(path)?;
    let text = match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "El archivo no es UTF-8 válido",
            ));
        }
    };

    let eol = detect_eol(&text);
    let segments = parse_note(&text);
    let disk_mtime = fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    Ok(LoadedNote {
        path: path.to_path_buf(),
        eol,
        segments,
        dirty: false,
        disk_mtime,
    })
}

pub fn save_note(note: &mut LoadedNote) -> Result<SystemTime, std::io::Error> {
    let text = note.full_text();
    let parent = note
        .path
        .parent()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Directorio no encontrado"))?;

    // Atomic write in the same directory using a temp file
    let mut temp_file = tempfile::NamedTempFile::new_in(parent)?;
    use std::io::Write;
    temp_file.write_all(text.as_bytes())?;
    temp_file.flush()?;

    // Persist replaces the destination file atomically
    temp_file.persist(&note.path).map_err(|e| e.error)?;

    let new_mtime = fs::metadata(&note.path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    note.disk_mtime = new_mtime;
    note.dirty = false;

    Ok(new_mtime)
}

