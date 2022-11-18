// Copyright © 2022 The Radicle Git Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Unit tests for radicle_surf::diff

use pretty_assertions::assert_eq;
use radicle_surf::diff::*;

/* TODO(fintan): Move is not detected yet
#[test]
fn test_moved_file() {
    let mut directory = Directory::root();
    directory.insert_file(&unsound::path::new("mod.rs"), File::new(b"use banana"));

    let mut new_directory = Directory::root();
    new_directory.insert_file(&unsound::path::new("banana.rs"), File::new(b"use banana"));

    let diff = Diff::diff(directory, new_directory).expect("diff failed");

    assert_eq!(diff, Diff::new())
}
*/

/* TODO(fintan): Tricky stuff
#[test]
fn test_disjoint_directories() {
    let mut directory = Directory::root();
    directory.insert_file(
        &unsound::path::new("foo/src/banana.rs"),
        File::new(b"use banana"),
    );

    let mut other_directory = Directory::root();
    other_directory.insert_file(
        &unsound::path::new("bar/src/pineapple.rs"),
        File::new(b"use pineapple"),
    );

    let diff = Diff::diff(directory, other_directory).expect("diff failed");

    let expected_diff = Diff {
        created: vec![CreateFile(Path::from_labels(
            unsound::label::new("bar"),
            &[
                unsound::label::new("src"),
                unsound::label::new("pineapple.rs"),
            ],
        ))],
        deleted: vec![DeleteFile(Path::from_labels(
            unsound::label::new("foo"),
            &[unsound::label::new("src"), unsound::label::new("banana.rs")],
        ))],
        moved: vec![],
        modified: vec![],
    };

    assert_eq!(diff, expected_diff)
}
*/

#[test]
fn test_both_missing_eof_newline() {
    let buf = r#"
diff --git a/.env b/.env
index f89e4c0..7c56eb7 100644
--- a/.env
+++ b/.env
@@ -1 +1 @@
-hello=123
\ No newline at end of file
+hello=1234
\ No newline at end of file
"#;
    let diff = git2::Diff::from_buffer(buf.as_bytes()).unwrap();
    let diff = Diff::try_from(diff).unwrap();
    assert_eq!(diff.modified[0].eof, Some(EofNewLine::BothMissing));
}

#[test]
fn test_none_missing_eof_newline() {
    let buf = r#"
diff --git a/.env b/.env
index f89e4c0..7c56eb7 100644
--- a/.env
+++ b/.env
@@ -1 +1 @@
-hello=123
+hello=1234
"#;
    let diff = git2::Diff::from_buffer(buf.as_bytes()).unwrap();
    let diff = Diff::try_from(diff).unwrap();
    assert_eq!(diff.modified[0].eof, None);
}

// TODO(xphoniex): uncomment once libgit2 has fixed the bug
//#[test]
//     fn test_old_missing_eof_newline() {
//         let buf = r#"
// diff --git a/.env b/.env
// index f89e4c0..7c56eb7 100644
// --- a/.env
// +++ b/.env
// @@ -1 +1 @@
// -hello=123
// \ No newline at end of file
// +hello=1234
// "#;
//         let diff = git2::Diff::from_buffer(buf.as_bytes()).unwrap();
//         let diff = Diff::try_from(diff).unwrap();
//         assert_eq!(diff.modified[0].eof, Some(EofNewLine::OldMissing));
//     }

// TODO(xphoniex): uncomment once libgit2 has fixed the bug
//#[test]
//     fn test_new_missing_eof_newline() {
//         let buf = r#"
// diff --git a/.env b/.env
// index f89e4c0..7c56eb7 100644
// --- a/.env
// +++ b/.env
// @@ -1 +1 @@
// -hello=123
// +hello=1234
// \ No newline at end of file
// "#;
//         let diff = git2::Diff::from_buffer(buf.as_bytes()).unwrap();
//         let diff = Diff::try_from(diff).unwrap();
//         assert_eq!(diff.modified[0].eof, Some(EofNewLine::NewMissing));
//     }
