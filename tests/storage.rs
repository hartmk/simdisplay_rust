use simdisplay::storage::*;
#[test]
fn restart_retains_workspace() {
    let t = tempfile::tempdir().unwrap();
    let path = t.path().join("state.json");
    let (s, mut w) = Store::load(path.clone());
    w.active.name = "My rig".into();
    w.library.insert("track".into(), w.active.clone());
    s.save(&w).unwrap();
    let (_, loaded) = Store::load(path);
    assert_eq!(loaded, w);
}
#[test]
fn corrupt_data_is_preserved_until_explicit_reset() {
    let t = tempfile::tempdir().unwrap();
    let path = t.path().join("state.json");
    std::fs::write(&path, b"broken").unwrap();
    let (mut s, w) = Store::load(path.clone());
    assert!(s.error.is_some());
    assert!(s.save(&w).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"broken");
    let backup = s.reset().unwrap().unwrap();
    assert_eq!(std::fs::read(backup).unwrap(), b"broken");
    s.save(&w).unwrap();
    assert!(Store::load(path).0.error.is_none());
}
#[test]
fn invalid_library_does_not_replace_valid_file() {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("state.json");
    let (s, mut w) = Store::load(p.clone());
    s.save(&w).unwrap();
    let old = std::fs::read(&p).unwrap();
    let mut bad = w.active.clone();
    bad.width = 1;
    w.library.insert("bad".into(), bad);
    assert!(s.save(&w).is_err());
    assert_eq!(std::fs::read(p).unwrap(), old);
}
#[cfg(unix)]
#[test]
fn permission_error_is_not_treated_as_missing_file() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let parent = temp.path().join("locked");
    std::fs::create_dir(&parent).unwrap();
    let path = parent.join("workspace.json");
    std::fs::write(&path, b"preserve me").unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o000)).unwrap();
    let (store, w) = Store::load(path.clone());
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(
        store.error.is_some(),
        "permission failure must block storage"
    );
    assert!(store.save(&w).is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"preserve me");
}
