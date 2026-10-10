use super::*;

fn entry(favorite: bool, tags: &[&str]) -> Entry {
    Entry {
        favorite,
        tags: tags.iter().map(|t| t.to_string()).collect(),
        ..Entry::default()
    }
}

#[test]
fn dates() {
    assert_eq!(civil_date(0), (1970, 1, 1));
    // 2026-10-06 14:03:00 UTC
    assert_eq!(
        date_and_time(1_791_295_380),
        ("2026-10-06".into(), "2026-10-06 14:03".into())
    );
    assert_eq!(civil_date(951_782_400), (2000, 2, 29));
}

#[test]
fn export_then_import_on_another_machine() {
    let mut old = Catalog::default();
    old.items
        .insert("/home/a/wpp/x/one.jpg".into(), entry(true, &["Mitsu"]));
    old.items.insert("/home/a/wpp/two.png".into(), entry(false, &["dark"]));
    old.items.insert("/home/a/wpp/three.jpg".into(), entry(true, &[]));
    old.items.insert("/home/a/wpp/gone.jpg".into(), entry(true, &[]));
    let sizes = |p: &Path| Some(if p.ends_with("three.jpg") { 300 } else { 100 });
    let backup = export(&old, &["/home/a/wpp".into()], "2026-10-06 14:03".into(), &sizes);
    assert_eq!(backup.items[0].relative.as_deref(), Some(Path::new("gone.jpg")));

    // New home, a different root; three.jpg moved to another subfolder.
    let folders = vec![PathBuf::from("/home/b/Pictures")];
    let library = vec![
        ("/home/b/Pictures/x/one.jpg".into(), 100),
        ("/home/b/Pictures/two.png".into(), 100),
        ("/home/b/Pictures/elsewhere/three.jpg".into(), 300),
        ("/home/b/Pictures/other/three.jpg".into(), 999),
    ];
    let mut now = Catalog::default();
    now.items
        .insert("/home/b/Pictures/two.png".into(), entry(false, &["Dark", "night"]));
    let report = import(&mut now, &backup, &folders, &library, false);
    assert_eq!(
        report,
        ImportReport {
            same_place: 0,
            moved: 3,
            missing: vec!["gone.jpg".into()],
            ..ImportReport::default()
        }
    );
    assert_eq!(
        now.items[Path::new("/home/b/Pictures/x/one.jpg")],
        entry(true, &["Mitsu"])
    );
    // Merged: tags once (case-insensitive).
    assert_eq!(
        now.items[Path::new("/home/b/Pictures/two.png")],
        entry(false, &["Dark", "night"])
    );
    // By name, told apart by size.
    assert!(now.items[Path::new("/home/b/Pictures/elsewhere/three.jpg")].favorite);
}

#[test]
fn metadata_goes_through_a_backup() {
    use crate::catalog::{Metadata, Origin, Person};
    let meta = Metadata {
        origin: Some(Origin {
            provider: "wallhaven".into(),
            id: "abc123".into(),
        }),
        artists: vec![Person {
            name: "Mitsu".into(),
            query: Some("id:1".into()),
        }],
        ..Metadata::default()
    };
    let mut old = Catalog::default();
    old.fill_metadata(Path::new("/w/a.jpg"), &meta);
    old.fill_metadata(Path::new("/w/b.jpg"), &meta);
    let backup = export(&old, &[], String::new(), &|_| None);
    // As a file, and read back by the format before metadata (no fields
    // for it): still a backup.
    let text = serde_json::to_string(&backup).unwrap();
    assert_eq!(serde_json::from_str::<Backup>(&text).unwrap(), backup);

    let mut now = Catalog::default();
    // b.jpg has an artist of its own already: kept, the backup's added.
    now.set_artists(Path::new("/w/b.jpg"), &[Person::named("Other")]);
    let library = vec![("/w/a.jpg".into(), 1), ("/w/b.jpg".into(), 1)];
    import(&mut now, &backup, &[], &library, false);
    assert_eq!(now.metadata(Path::new("/w/a.jpg")), Some(&meta));
    let b = now.metadata(Path::new("/w/b.jpg")).unwrap();
    assert_eq!(
        b.artists.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
        ["Other", "Mitsu"]
    );
    assert_eq!(b.origin, meta.origin);
}

#[test]
fn replace_keeps_only_the_backup() {
    let mut old = Catalog::default();
    old.items.insert("/w/a.jpg".into(), entry(true, &[]));
    let backup = export(&old, &[], String::new(), &|_| None);
    let mut now = Catalog::default();
    now.items.insert("/w/b.jpg".into(), entry(true, &[]));
    let library = vec![("/w/a.jpg".into(), 1), ("/w/b.jpg".into(), 1)];
    let report = import(&mut now, &backup, &[], &library, true);
    assert_eq!(report.same_place, 1);
    assert_eq!(now.items.keys().collect::<Vec<_>>(), vec![Path::new("/w/a.jpg")]);
}
