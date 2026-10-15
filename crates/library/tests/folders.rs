use goymusic_core::FolderId;
use goymusic_library::db::Db;
use goymusic_library::error::LibraryError;
use goymusic_library::folders;

#[test]
fn add_stores_canonical_path_and_lists_it() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open_in_memory().unwrap();
    let with_dots = dir.path().join(".");
    let folder = folders::add(&db, &with_dots).unwrap();
    let canonical = std::fs::canonicalize(dir.path()).unwrap();
    assert_eq!(folder.path, canonical.to_str().unwrap());
    assert_eq!(folders::list(&db).unwrap(), vec![folder]);
}

#[test]
fn overlapping_folders_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let child = dir.path().join("child");
    std::fs::create_dir(&child).unwrap();
    let db = Db::open_in_memory().unwrap();
    folders::add(&db, &child).unwrap();

    assert!(matches!(folders::add(&db, &child), Err(LibraryError::FolderOverlap(_))));
    assert!(matches!(
        folders::add(&db, dir.path()),
        Err(LibraryError::FolderOverlap(_))
    ));
    let grandchild = child.join("deeper");
    std::fs::create_dir(&grandchild).unwrap();
    assert!(matches!(
        folders::add(&db, &grandchild),
        Err(LibraryError::FolderOverlap(_))
    ));
}

#[test]
fn missing_path_is_an_io_error() {
    let db = Db::open_in_memory().unwrap();
    assert!(matches!(
        folders::add(&db, std::path::Path::new("/definitely/not/here")),
        Err(LibraryError::Io(_))
    ));
}

#[test]
fn remove_unknown_folder_is_not_found() {
    let mut db = Db::open_in_memory().unwrap();
    assert!(matches!(
        folders::remove(&mut db, FolderId(42)),
        Err(LibraryError::NotFound { kind: "folder", id: 42 })
    ));
}
