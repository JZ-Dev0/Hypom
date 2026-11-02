use std::fs;
use tempfile::tempdir;
use hypom_storage::{
    scan_workspace, create_note, create_project, load_note, save_note,
    AutosaveManager, SearchIndex,
};

#[test]
fn test_workspace_scanning_and_creation() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // Create a general note
    create_note(root, None, "ideas").unwrap();

    // Create a project and note
    create_project(root, "servidores").unwrap();
    create_note(root, Some("servidores"), "ftp").unwrap();

    // Add metadata
    let meta_path = root.join("servidores").join("proyecto.toml");
    fs::write(
        meta_path,
        "nombre = \"Mis Servidores\"\ncolor = \"#ff0000\"\norden = 1\n",
    ).unwrap();

    let ws = scan_workspace(root).unwrap();
    assert_eq!(ws.general.len(), 1);
    assert_eq!(ws.general[0].title, "ideas");

    assert_eq!(ws.projects.len(), 1);
    assert_eq!(ws.projects[0].name, "servidores");
    assert_eq!(ws.projects[0].display_name(), "Mis Servidores");
    assert_eq!(ws.projects[0].notes.len(), 1);
    assert_eq!(ws.projects[0].notes[0].title, "ftp");
}

#[test]
fn test_note_save_and_reload() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let note_path = create_note(root, None, "test_note").unwrap();
    let sample_text = "@tabla ip, user\n192.168.1.1 admin\n@fin";
    fs::write(&note_path, sample_text).unwrap();

    let mut loaded = load_note(&note_path).unwrap();
    assert_eq!(loaded.segments.len(), 1);

    // Save
    save_note(&mut loaded).unwrap();
    assert!(!loaded.dirty);

    let reloaded = load_note(&note_path).unwrap();
    assert_eq!(reloaded.full_text(), sample_text);
}

#[test]
fn test_autosave_manager() {
    let mut manager = AutosaveManager::new(50);
    let path = std::path::PathBuf::from("dummy.txt");

    manager.mark_modified(path.clone());
    assert!(!manager.check_should_save(&path));

    std::thread::sleep(std::time::Duration::from_millis(60));
    assert!(manager.check_should_save(&path));
}

#[test]
fn test_search_index_excludes_secrets() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let note_path = create_note(root, None, "credentials").unwrap();
    let content = "@registro \"Cuentas\" user, pass:secreto\njuan supersecretpassword123\n@fin";
    fs::write(&note_path, content).unwrap();

    let ws = scan_workspace(root).unwrap();
    let mut index = SearchIndex::new();
    index.build_from_workspace(&ws);

    let found_secret = index.entries.iter().any(|e| e.text.contains("supersecretpassword123"));
    assert!(!found_secret, "Search index must not contain secrets!");

    let found_user = index.entries.iter().any(|e| e.text.contains("juan"));
    assert!(found_user, "Search index should contain non-secret cells");
}
