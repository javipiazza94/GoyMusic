//! The committed `ui/src/bindings.ts` must match the Rust commands and types, so an
//! incompatible change fails here (and in `tsc`) instead of at runtime.

#[test]
fn bindings_are_up_to_date() {
    let out = tempfile::NamedTempFile::new().unwrap();
    goymusic_desktop::export_bindings(out.path()).unwrap();
    let fresh = std::fs::read_to_string(out.path()).unwrap();
    let committed_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/bindings.ts");
    if std::env::var_os("UPDATE_BINDINGS").is_some() {
        std::fs::write(committed_path, &fresh).unwrap();
        return;
    }
    assert_eq!(
        std::fs::read_to_string(committed_path).unwrap_or_default(),
        fresh,
        "ui/src/bindings.ts is stale: run UPDATE_BINDINGS=1 cargo test -p goymusic-desktop --test bindings"
    );
}
