use super::{bundle, Error};
use std::io::Cursor;
fn archive(entries: &[(&str, tar::EntryType, &[u8], Option<&str>)]) -> Vec<u8> {
    let mut tar = tar::Builder::new(flate2::write::GzEncoder::new(
        Vec::new(),
        flate2::Compression::default(),
    ));
    for (name, kind, bytes, link) in entries {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(*kind);
        header.set_size(bytes.len() as u64);
        header.set_mode(0o755);
        if let Some(link) = link {
            header.set_link_name(link).unwrap();
        }
        header.set_cksum();
        tar.append_data(&mut header, name, Cursor::new(bytes))
            .unwrap();
    }
    tar.into_inner().unwrap().finish().unwrap()
}
#[test]
fn archive_namespace_rejects_duplicates_case_aliases_second_bundles_links_and_devices() {
    use tar::EntryType as T;
    for entries in [
        vec![
            ("A.app/file", T::Regular, &b"one"[..], None),
            ("A.app/file", T::Regular, &b"two"[..], None),
        ],
        vec![
            ("A.app/File", T::Regular, &b"one"[..], None),
            ("A.app/file", T::Regular, &b"two"[..], None),
        ],
        vec![
            ("A.app/file", T::Regular, &b"one"[..], None),
            ("B.app/file", T::Regular, &b"two"[..], None),
        ],
        vec![("A.app/link", T::Symlink, &b""[..], Some("../../outside"))],
        vec![("A.app/link", T::Symlink, &b""[..], Some("/absolute"))],
        vec![("A.app/link", T::Link, &b""[..], Some("A.app/file"))],
        vec![("A.app/pipe", T::Fifo, &b""[..], None)],
        vec![
            ("A.app/link", T::Symlink, &b""[..], Some("dir")),
            ("A.app/link/file", T::Regular, &b""[..], None),
        ],
    ] {
        let root = tempfile::tempdir().unwrap();
        let stage = root.path().join("stage");
        assert_eq!(
            bundle::extract(&archive(&entries), &stage, uuid::Uuid::new_v4()),
            Err(Error::Package)
        );
        assert!(!stage.exists(), "preflight must reject before writes");
    }
}
#[test]
#[cfg(unix)]
fn contained_framework_symlink_is_supported_and_target_ancestry_links_are_rejected() {
    use tar::EntryType as T;
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let stage = root.join("stage");
    let bytes = archive(&[
        (
            "A.app/Framework/Versions/A/binary",
            T::Regular,
            b"library",
            None,
        ),
        (
            "A.app/Framework/Versions/Current",
            T::Symlink,
            b"",
            Some("A"),
        ),
    ]);
    let bundle = bundle::extract(&bytes, &stage, uuid::Uuid::new_v4()).unwrap();
    assert_eq!(
        std::fs::read(bundle.join("Framework/Versions/Current/binary")).unwrap(),
        b"library"
    );
    std::os::unix::fs::symlink(&bundle, root.join("alias.app")).unwrap();
    assert_eq!(bundle::plain(&root.join("alias.app")), Err(Error::Target));
}

#[test]
fn tree_fingerprint_frames_each_directory_and_cannot_confuse_sibling_with_descendant() {
    let root = tempfile::tempdir().unwrap();
    let a = root.path().join("A");
    let b = root.path().join("B");
    std::fs::create_dir_all(a.join("a")).unwrap();
    std::fs::create_dir_all(b.join("a")).unwrap();
    std::fs::write(a.join("a/x"), b"same x").unwrap();
    std::fs::write(b.join("a/x"), b"same x").unwrap();
    std::fs::write(a.join("y"), b"same y").unwrap();
    std::fs::write(b.join("a/y"), b"same y").unwrap();
    assert_ne!(
        bundle::fingerprint(&a).unwrap(),
        bundle::fingerprint(&b).unwrap()
    );
}
