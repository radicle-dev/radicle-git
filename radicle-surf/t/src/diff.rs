use pretty_assertions::assert_eq;
use radicle_git_ext::{ref_format::refname, Oid};
use radicle_surf::{
    diff::{
        Added,
        Diff,
        DiffContent,
        DiffFile,
        EofNewLine,
        FileDiff,
        FileMode,
        Hunk,
        Line,
        Modification,
        Modified,
        Stats,
    },
    Branch,
    Error,
    Repository,
};
use std::{path::Path, str::FromStr};

use super::GIT_PLATINUM;

#[test]
fn test_initial_diff() -> Result<(), Error> {
    let repo = Repository::open(GIT_PLATINUM)?;
    let oid = Oid::from_str("d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3")?;
    let commit = repo.commit(oid).unwrap();
    assert!(commit.parents.is_empty());

    let diff = repo.diff_commit(oid)?;
    let diff_stats = *diff.stats();
    let diff_files = diff.into_files();

    let expected_files = vec![FileDiff::Added(Added {
        path: Path::new("README.md").to_path_buf(),
        diff: DiffContent::Plain {
            hunks: vec![Hunk {
                header: Line::from(b"@@ -0,0 +1 @@\n".to_vec()),
                lines: vec![Modification::addition(
                    b"This repository is a data source for the Upstream front-end tests.\n"
                        .to_vec(),
                    1,
                )],
                old: 0..0,
                new: 1..2,
            }]
            .into(),
            eof: EofNewLine::default(),
        },
        new: DiffFile {
            oid: Oid::from_str("7f48df0118b1674f4ab0ed1717c1368091a5dddc").unwrap(),
            mode: FileMode::Blob,
        },
    })];

    let expected_stats = Stats {
        files_changed: 1,
        insertions: 1,
        deletions: 0,
    };

    assert_eq!(expected_files, diff_files);
    assert_eq!(expected_stats, diff_stats);

    Ok(())
}

#[test]
fn test_diff_of_rev() -> Result<(), Error> {
    let repo = Repository::open(GIT_PLATINUM)?;
    let diff = repo.diff_commit("80bacafba303bf0cdf6142921f430ff265f25095")?;
    assert_eq!(diff.files().count(), 1);
    Ok(())
}

#[test]
fn test_diff_file() -> Result<(), Error> {
    let repo = Repository::open(GIT_PLATINUM)?;
    let path_buf = Path::new("README.md").to_path_buf();
    let diff = repo.diff_file(
        &path_buf,
        "d6880352fc7fda8f521ae9b7357668b17bb5bad5",
        "223aaf87d6ea62eef0014857640fd7c8dd0f80b5",
    )?;
    let expected_diff = FileDiff::Modified(Modified {
        path: path_buf,
        diff: DiffContent::Plain {
            hunks: vec![Hunk {
                header: Line::from(b"@@ -1 +1,2 @@\n".to_vec()),
                lines: vec![
                    Modification::deletion(b"This repository is a data source for the Upstream front-end tests.\n".to_vec(), 1),
                    Modification::addition(b"This repository is a data source for the Upstream front-end tests and the\n".to_vec(), 1),
                    Modification::addition(b"[`radicle-surf`](https://github.com/radicle-dev/git-platinum) unit tests.\n".to_vec(), 2),
                ],
                old: 1..2,
                new: 1..3,
            }]
            .into(),
            eof: EofNewLine::default(),
        },
        old: DiffFile {
            oid: Oid::from_str("7f48df0118b1674f4ab0ed1717c1368091a5dddc").unwrap(),
            mode: FileMode::Blob,
        },
        new: DiffFile {
            oid: Oid::from_str("5e07534cd74a6a9b2ccd2729b181c4ef26173a5e").unwrap(),
            mode: FileMode::Blob,
        },
    });
    assert_eq!(expected_diff, diff);

    Ok(())
}

#[test]
fn test_diff() -> Result<(), Error> {
    let repo = Repository::open(GIT_PLATINUM)?;
    let oid = "80bacafba303bf0cdf6142921f430ff265f25095";
    let commit = repo.commit(oid).unwrap();
    let parent_oid = commit.parents.get(0).unwrap();
    let diff = repo.diff(*parent_oid, oid)?;

    let expected_files = vec![FileDiff::Modified(Modified {
        path: Path::new("README.md").to_path_buf(),
        diff: DiffContent::Plain {
            hunks: vec![Hunk {
                header: Line::from(b"@@ -1 +1,2 @@\n".to_vec()),
                lines: vec![
                    Modification::deletion(b"This repository is a data source for the Upstream front-end tests.\n".to_vec(), 1),
                    Modification::addition(b"This repository is a data source for the Upstream front-end tests and the\n".to_vec(), 1),
                    Modification::addition(b"[`radicle-surf`](https://github.com/radicle-dev/git-platinum) unit tests.\n".to_vec(), 2),
                ],
                old: 1..2,
                new: 1..3,
            }]
            .into(),
            eof: EofNewLine::default(),
        },
        old: DiffFile {
            oid: Oid::from_str("7f48df0118b1674f4ab0ed1717c1368091a5dddc").unwrap(),
            mode: FileMode::Blob,
        },
        new: DiffFile {
            oid: Oid::from_str("5e07534cd74a6a9b2ccd2729b181c4ef26173a5e").unwrap(),
            mode: FileMode::Blob,
        },
    })];
    let expected_stats = Stats {
        files_changed: 1,
        insertions: 2,
        deletions: 1,
    };
    let diff_stats = *diff.stats();
    let diff_files = diff.into_files();
    assert_eq!(expected_files, diff_files);
    assert_eq!(expected_stats, diff_stats);

    Ok(())
}

#[test]
fn test_branch_diff() -> Result<(), Error> {
    let repo = Repository::open(GIT_PLATINUM)?;
    let rev_from = Branch::local(refname!("master"));
    let rev_to = Branch::local(refname!("dev"));
    let diff = repo.diff(&rev_from, &rev_to)?;

    println!("Diff two branches: master -> dev");
    println!(
        "result: added {} deleted {} moved {} modified {}",
        diff.added().count(),
        diff.deleted().count(),
        diff.moved().count(),
        diff.modified().count()
    );
    assert_eq!(diff.added().count(), 1);
    assert_eq!(diff.deleted().count(), 11);
    assert_eq!(diff.moved().count(), 1);
    assert_eq!(diff.modified().count(), 2);
    for c in diff.added() {
        println!("added: {:?}", &c.path);
    }
    for d in diff.deleted() {
        println!("deleted: {:?}", &d.path);
    }
    for m in diff.moved() {
        println!("moved: {:?} -> {:?}", &m.old_path, &m.new_path);
    }
    for m in diff.modified() {
        println!("modified: {:?}", &m.path);
    }

    // Verify moved.
    let diff_moved = diff.moved().next().unwrap();

    // We can find a `FileDiff` for the old_path in a move.
    let file_diff = repo.diff_file(&diff_moved.old_path, &rev_from, &rev_to)?;
    println!("old path file diff: {:?}", &file_diff);

    // We can find a `FileDiff` for the new_path in a move.
    let file_diff = repo.diff_file(&diff_moved.new_path, &rev_from, &rev_to)?;
    println!("new path file diff: {:?}", &file_diff);

    // We can find a `FileDiff` if given a directory name.
    let dir_diff = repo.diff_file(&"special/", &rev_from, &rev_to)?;
    println!("dir diff: {dir_diff:?}");

    Ok(())
}

#[test]
fn test_diff_serde() -> Result<(), Error> {
    let repo = Repository::open(GIT_PLATINUM)?;
    let rev_from = Branch::local(refname!("master"));
    let rev_to = Branch::local(refname!("diff-test"));
    let diff = repo.diff(rev_from, rev_to)?;

    let json = serde_json::json!({
        "added": [{
            "path": "LICENSE",
            "diff": {
                "type": "plain",
                "hunks": [{
                    "header": "@@ -0,0 +1,2 @@\n",
                    "lines": [{
                        "line": "This is a license file.\n",
                        "lineNo": 1,
                        "type": "addition",
                    },
                    {
                        "line": "\n",
                        "lineNo": 2,
                        "type": "addition",
                    }],
                    "old": { "start": 0, "end": 0 },
                    "new": { "start": 1, "end": 3 },
                }],
                "eof": "noneMissing",
            },
            "new": {
                "mode": "blob",
                "oid": "02f70f56ec62396ceaf38804c37e169e875ab291",
            },
        }],
        "deleted": [{
            "path": "text/arrows.txt",
            "old": {
                "mode": "blob",
                "oid": "95418c04010a3cc758fb3a37f9918465f147566f",
             },
            "diff": {
                "type": "plain",
                "hunks": [{
                    "header": "@@ -1,7 +0,0 @@\n",
                    "lines": [
                        {
                            "line": "  ;;;;;        ;;;;;        ;;;;;\n",
                            "lineNo": 1,
                            "type": "deletion",
                        },
                        {
                            "line": "  ;;;;;        ;;;;;        ;;;;;\n",
                            "lineNo": 2,
                            "type": "deletion",
                        },
                        {
                            "line": "  ;;;;;        ;;;;;        ;;;;;\n",
                            "lineNo": 3,
                            "type": "deletion",
                        },
                        {
                            "line": "  ;;;;;        ;;;;;        ;;;;;\n",
                            "lineNo": 4,
                            "type": "deletion",
                        },
                        {
                            "line": "..;;;;;..    ..;;;;;..    ..;;;;;..\n",
                            "lineNo": 5,
                            "type": "deletion",
                        },
                        {
                            "line": " ':::::'      ':::::'      ':::::'\n",
                            "lineNo": 6,
                            "type": "deletion",
                        },
                        {
                            "line": "   ':`          ':`          ':`\n",
                            "lineNo": 7,
                            "type": "deletion",
                        },
                    ],
                    "old": { "start": 1, "end": 8 },
                    "new": { "start": 0, "end": 0 },
                }],
                "eof": "noneMissing",
            },
        }],
        "moved": [{
            "oldPath": "text/emoji.txt",
            "newPath": "emoji.txt",
        }],
        "copied": [{
            "diff": {
                "type": "empty",
            },
            "newPath": "file_operations/copied.md",
            "oldPath": "README.md",
        }],
        "modified": [{
            "path": "README.md",
            "diff": {
                "type": "plain",
                "hunks": [{
                    "header": "@@ -1,2 +1,2 @@\n",
                    "lines": [
                        { "lineNo": 1,
                          "line": "This repository is a data source for the Upstream front-end tests and the\n",
                          "type": "deletion"
                        },
                        { "lineNo": 2,
                          "line": "[`radicle-surf`](https://github.com/radicle-dev/git-platinum) unit tests.\n",
                          "type": "deletion"
                        },
                        { "lineNo": 1,
                          "line": "This repository is a data source for the upstream front-end tests and the\n",
                          "type": "addition"
                        },
                        { "lineNo": 2,
                          "line": "[`radicle-surf`](https://github.com/radicle-dev/radicle-surf) unit tests.\n",
                          "type": "addition"
                        },
                    ],
                    "old": { "start": 1, "end": 3 },
                    "new": { "start": 1, "end": 3 },
                }],
                "eof": "noneMissing",
            },
            "new": {
                "mode": "blob",
                "oid": "b033ecf407a44922b28c942c696922a7d7daf06e",
            },
            "old": {
                "mode": "blob",
                "oid": "5e07534cd74a6a9b2ccd2729b181c4ef26173a5e",
            },
        }],
        "stats": {
            "deletions": 9,
            "filesChanged": 5,
            "insertions": 4,
        }
    });
    assert_eq!(serde_json::to_value(diff).unwrap(), json);

    Ok(())
}

// A possible false positive is being hit here for this clippy
// warning. Tracking issue:
// https://github.com/rust-lang/rust-clippy/issues/11402
#[allow(clippy::needless_raw_string_hashes)]
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
    assert_eq!(
        diff.modified().next().unwrap().diff.eof(),
        Some(EofNewLine::BothMissing)
    );
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
    assert_eq!(
        diff.modified().next().unwrap().diff.eof(),
        Some(EofNewLine::NoneMissing)
    );
}

// TODO(xphoniex): uncomment once libgit2 has fixed the bug
//#[test]
//fn test_old_missing_eof_newline() {
//    let buf = r#"
//diff --git a/.env b/.env
//index f89e4c0..7c56eb7 100644
//--- a/.env
//+++ b/.env
//@@ -1 +1 @@
//-hello=123
//\ No newline at end of file
//+hello=1234
//"#;
//    let diff = git2::Diff::from_buffer(buf.as_bytes()).unwrap();
//    let diff = Diff::try_from(diff).unwrap();
//    assert_eq!(
//        diff.modified().next().unwrap().diff.eof(),
//        Some(EofNewLine::OldMissing)
//    );
//}

// TODO(xphoniex): uncomment once libgit2 has fixed the bug
//#[test]
//fn test_new_missing_eof_newline() {
//    let buf = r#"
//diff --git a/.env b/.env
//index f89e4c0..7c56eb7 100644
//--- a/.env
//+++ b/.env
//@@ -1 +1 @@
//-hello=123
//+hello=1234
//\ No newline at end of file
//"#;
//    let diff = git2::Diff::from_buffer(buf.as_bytes()).unwrap();
//    let diff = Diff::try_from(diff).unwrap();
//    assert_eq!(
//        diff.modified().next().unwrap().diff.eof(),
//        Some(EofNewLine::NewMissing)
//    );
//}
