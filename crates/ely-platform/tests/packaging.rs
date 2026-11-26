// The installer reads these columns; the table is the contract.

#[test]
fn packaging_table_lists_what_the_installer_reads() {
    let table = include_str!("../../../packaging/apps.tsv");
    // (slug, name, categories, batch, licence).
    let expected = [
        ("ely-doc", "ElyDoc", "Office;WordProcessor;", "1", "MIT AND MPL-2.0"),
        ("ely-table", "ElyTable", "Office;Spreadsheet;", "1", "MIT AND MPL-2.0"),
        ("ely-show", "ElyShow", "Office;Presentation;", "1", "MIT AND MPL-2.0"),
        ("ely-pdf", "ElyPDF", "Office;Viewer;", "1", "MIT AND AGPL-3.0-or-later"),
        ("ely-mail", "ElyMail", "Network;Email;", "1", "MIT"),
        ("ely-tree", "ElyTree", "Development;RevisionControl;", "1", "MIT AND GPL-2.0-only"),
        ("green", "Green", "Utility;TextEditor;", "1", "GPL-3.0-or-later"),
        ("ely-agent", "ElyAgent", "Development;", "2", "MIT"),
    ];
    let rows: Vec<Vec<&str>> = table.lines().map(|line| line.split('\t').collect()).collect();
    assert_eq!(rows.len(), expected.len(), "the table has {} rows", rows.len());
    for (row, want) in rows.iter().zip(expected) {
        assert_eq!(row.len(), 6, "{} has {} columns", row[0], row.len());
        assert_eq!((row[0], row[1], row[2], row[3], row[4]), want, "{}", row[0]);
        assert!(!row[5].is_empty(), "{} has no summary", row[0]);
    }
}

/// Green is the app this repository builds, and it ships under the workspace licence.
#[test]
fn green_ships_under_the_workspace_licence() {
    let table = include_str!("../../../packaging/apps.tsv");
    let row = table.lines().find(|line| line.starts_with("green\t")).expect("green's row");
    let licence = row.split('\t').nth(4).expect("a licence column");
    let manifest = include_str!("../../../Cargo.toml");
    let declared = manifest
        .lines()
        .find_map(|line| line.strip_prefix("license = \""))
        .map(|rest| rest.trim_end_matches('"'))
        .expect("a workspace licence");
    assert_eq!(licence, declared, "packaging/apps.tsv and Cargo.toml disagree on Green's licence");
}
