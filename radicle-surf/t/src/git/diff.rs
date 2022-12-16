use git_ref_format::refname;
use pretty_assertions::assert_eq;
use radicle_git_ext::Oid;
use radicle_surf::{
    diff::{
        Added,
        Addition,
        Diff,
        EofNewLine,
        FileDiff,
        Hunk,
        Hunks,
        Line,
        Modification,
        Modified,
        Moved,
        Stats,
    },
    git::{Branch, Error, Repository},
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

    let expected_diff = Diff {
        added: vec![Added {
            path: Path::new("README.md").to_path_buf(),
            diff: FileDiff::Plain {
                hunks: vec![Hunk {
                    header: Line::from(b"@@ -0,0 +1 @@\n".to_vec()),
                    lines: vec![Addition {
                        line:
                            b"This repository is a data source for the Upstream front-end tests.\n"
                                .to_vec()
                                .into(),
                        line_no: 1,
                    }],
                }]
                .into(),
            },
        }],
        deleted: vec![],
        moved: vec![],
        copied: vec![],
        modified: vec![],
        stats: Stats {
            files_changed: 1,
            insertions: 1,
            deletions: 0,
        },
    };
    assert_eq!(expected_diff, diff);

    Ok(())
}

#[test]
fn test_diff_of_rev() -> Result<(), Error> {
    let repo = Repository::open(GIT_PLATINUM)?;
    let diff = repo.diff_commit("80bacafba303bf0cdf6142921f430ff265f25095")?;
    assert_eq!(diff.added.len(), 0);
    assert_eq!(diff.deleted.len(), 0);
    assert_eq!(diff.moved.len(), 0);
    assert_eq!(diff.modified.len(), 1);
    Ok(())
}

#[test]
fn test_diff() -> Result<(), Error> {
    let repo = Repository::open(GIT_PLATINUM)?;
    let oid = "80bacafba303bf0cdf6142921f430ff265f25095";
    let commit = repo.commit(oid).unwrap();
    let parent_oid = commit.parents.get(0).unwrap();
    let diff = repo.diff(*parent_oid, oid)?;

    let expected_diff = Diff {
        added: vec![],
        deleted: vec![],
        moved: vec![],
        copied: vec![],
        modified: vec![Modified {
            path: Path::new("README.md").to_path_buf(),
            diff: FileDiff::Plain {
                hunks: vec![Hunk {
                    header: Line::from(b"@@ -1 +1,2 @@\n".to_vec()),
                    lines: vec![
                        Modification::deletion(b"This repository is a data source for the Upstream front-end tests.\n".to_vec(), 1),
                        Modification::addition(b"This repository is a data source for the Upstream front-end tests and the\n".to_vec(), 1),
                        Modification::addition(b"[`radicle-surf`](https://github.com/radicle-dev/git-platinum) unit tests.\n".to_vec(), 2),
                    ]
                }].into()
            },
            eof: None,
        }],
        stats: Stats {
            files_changed: 1,
            insertions: 2,
            deletions: 1,
        },
    };
    assert_eq!(expected_diff, diff);

    Ok(())
}

#[test]
fn test_branch_diff() -> Result<(), Error> {
    let repo = Repository::open(GIT_PLATINUM)?;
    let diff = repo.diff(
        Branch::local(refname!("master")),
        Branch::local(refname!("dev")),
    )?;

    println!("Diff two branches: master -> dev");
    println!(
        "result: added {} deleted {} moved {} modified {}",
        diff.added.len(),
        diff.deleted.len(),
        diff.moved.len(),
        diff.modified.len()
    );
    assert_eq!(diff.added.len(), 1);
    assert_eq!(diff.deleted.len(), 11);
    assert_eq!(diff.moved.len(), 1);
    assert_eq!(diff.modified.len(), 2);
    for c in diff.added.iter() {
        println!("added: {:?}", &c.path);
    }
    for d in diff.deleted.iter() {
        println!("deleted: {:?}", &d.path);
    }
    for m in diff.moved.iter() {
        println!("moved: {:?} -> {:?}", &m.old_path, &m.new_path);
    }
    for m in diff.modified.iter() {
        println!("modified: {:?}", &m.path);
    }
    Ok(())
}

#[test]
fn test_diff_serde() {
    let diff = Diff {
        added: vec![ Added {
            path: Path::new("LICENSE").to_path_buf(),
            diff: FileDiff::Plain { hunks: Hunks::default() }
        }],
        deleted: vec![],
        moved: vec![ Moved {
            old_path: Path::new("CONTRIBUTING").to_path_buf(),
            new_path: Path::new("CONTRIBUTING.md").to_path_buf(),
        }],
        copied: vec![],
        modified: vec![ Modified {
            path: Path::new("README.md").to_path_buf(),
            diff: FileDiff::Plain {
                hunks: vec![Hunk {
                header: Line::from(b"@@ -1 +1,2 @@\n".to_vec()),
                lines: vec![
                    Modification::deletion(b"This repository is a data source for the Upstream front-end tests.\n".to_vec(), 1),
                    Modification::addition(b"This repository is a data source for the Upstream front-end tests and the\n".to_vec(), 1),
                    Modification::addition(b"[`radicle-surf`](https://github.com/radicle-dev/git-platinum) unit tests.\n".to_vec(), 2),
                    Modification::context(b"\n".to_vec(), 3, 4),
                ]
                }].into()
            },
            eof: None,
        }],
        stats: Stats {
            files_changed: 3,
            insertions: 2,
            deletions: 1,
        },
    };

    let eof: Option<u8> = None;
    let json = serde_json::json!({
        "added": [{"path": "LICENSE", "diff": {
                "type": "plain",
                "hunks": []
            },
        }],
        "deleted": [],
        "moved": [{ "oldPath": "CONTRIBUTING", "newPath": "CONTRIBUTING.md" }],
        "copied": [],
        "modified": [{
            "path": "README.md",
            "diff": {
                "type": "plain",
                "hunks": [{
                    "header": "@@ -1 +1,2 @@\n",
                    "lines": [
                        { "lineNo": 1,
                          "line": "This repository is a data source for the Upstream front-end tests.\n",
                          "type": "deletion"
                        },
                        { "lineNo": 1,
                          "line": "This repository is a data source for the Upstream front-end tests and the\n",
                          "type": "addition"
                        },
                        { "lineNo": 2,
                          "line": "[`radicle-surf`](https://github.com/radicle-dev/git-platinum) unit tests.\n",
                          "type": "addition"
                        },
                        { "lineNoOld": 3, "lineNoNew": 4,
                          "line": "\n",
                          "type": "context"
                        }
                    ]
                }]
            },
            "eof" : eof,
        }],
        "stats": {
            "deletions": 1,
            "filesChanged": 3,
            "insertions": 2,
        }
    });
    assert_eq!(serde_json::to_value(&diff).unwrap(), json);
}

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
