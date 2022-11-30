use git_ref_format::refname;
use pretty_assertions::assert_eq;
use radicle_git_ext::Oid;
use radicle_surf::{
    diff::{CreateFile, Diff, FileDiff, Hunk, Line, LineDiff, ModifiedFile},
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

    let diff = repo.initial_diff(oid)?;

    let expected_diff = Diff {
        created: vec![CreateFile {
            path: Path::new("README.md").to_path_buf(),
            diff: FileDiff::Plain {
                hunks: vec![Hunk {
                    header: Line::from(b"@@ -0,0 +1 @@\n".to_vec()),
                    lines: vec![LineDiff::addition(
                        b"This repository is a data source for the Upstream front-end tests.\n"
                            .to_vec(),
                        1,
                    )],
                }]
                .into(),
            },
        }],
        deleted: vec![],
        moved: vec![],
        copied: vec![],
        modified: vec![],
    };
    assert_eq!(expected_diff, diff);

    Ok(())
}

#[test]
fn test_diff_of_rev() -> Result<(), Error> {
    let repo = Repository::open(GIT_PLATINUM)?;
    let diff = repo.diff_from_parent("80bacafba303bf0cdf6142921f430ff265f25095")?;
    assert_eq!(diff.created.len(), 0);
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
                created: vec![],
                deleted: vec![],
                moved: vec![],
                copied: vec![],
                modified: vec![ModifiedFile {
                    path: Path::new("README.md").to_path_buf(),
                    diff: FileDiff::Plain {
                        hunks: vec![Hunk {
                            header: Line::from(b"@@ -1 +1,2 @@\n".to_vec()),
                            lines: vec![
                                LineDiff::deletion(b"This repository is a data source for the Upstream front-end tests.\n".to_vec(), 1),
                                LineDiff::addition(b"This repository is a data source for the Upstream front-end tests and the\n".to_vec(), 1),
                                LineDiff::addition(b"[`radicle-surf`](https://github.com/radicle-dev/git-platinum) unit tests.\n".to_vec(), 2),
                            ]
                        }].into()
                    },
                    eof: None,
                }]
            };
    assert_eq!(expected_diff, diff);

    Ok(())
}

#[test]
fn test_branch_diff() -> Result<(), Error> {
    let repo = Repository::open(GIT_PLATINUM)?;
    let diff = repo.diff(
        &Branch::local(refname!("master")),
        &Branch::local(refname!("dev")),
    )?;

    println!("Diff two branches: master -> dev");
    println!(
        "result: created {} deleted {} moved {} modified {}",
        diff.created.len(),
        diff.deleted.len(),
        diff.moved.len(),
        diff.modified.len()
    );
    assert_eq!(diff.created.len(), 1);
    assert_eq!(diff.deleted.len(), 11);
    assert_eq!(diff.moved.len(), 1);
    assert_eq!(diff.modified.len(), 2);
    for c in diff.created.iter() {
        println!("created: {:?}", &c.path);
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
    use radicle_surf::diff::{Hunks, MoveFile};

    let diff = Diff {
        created: vec![ CreateFile {
            path: Path::new("LICENSE").to_path_buf(),
            diff: FileDiff::Plain { hunks: Hunks::default() }
        }],
        deleted: vec![],
        moved: vec![ MoveFile {
            old_path: Path::new("CONTRIBUTING").to_path_buf(),
            new_path: Path::new("CONTRIBUTING.md").to_path_buf(),
        }],
        copied: vec![],
        modified: vec![ ModifiedFile {
            path: Path::new("README.md").to_path_buf(),
            diff: FileDiff::Plain {
                hunks: vec![Hunk {
                header: Line::from(b"@@ -1 +1,2 @@\n".to_vec()),
                lines: vec![
                    LineDiff::deletion(b"This repository is a data source for the Upstream front-end tests.\n".to_vec(), 1),
                    LineDiff::addition(b"This repository is a data source for the Upstream front-end tests and the\n".to_vec(), 1),
                    LineDiff::addition(b"[`radicle-surf`](https://github.com/radicle-dev/git-platinum) unit tests.\n".to_vec(), 2),
                    LineDiff::context(b"\n".to_vec(), 3, 4),
                ]
                }].into()
            },
            eof: None,
        }]
    };

    let eof: Option<u8> = None;
    let json = serde_json::json!({
        "created": [{"path": "LICENSE", "diff": {
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
                        { "lineNum": 1,
                          "line": "This repository is a data source for the Upstream front-end tests.\n",
                          "type": "deletion"
                        },
                        { "lineNum": 1,
                          "line": "This repository is a data source for the Upstream front-end tests and the\n",
                          "type": "addition"
                        },
                        { "lineNum": 2,
                          "line": "[`radicle-surf`](https://github.com/radicle-dev/git-platinum) unit tests.\n",
                          "type": "addition"
                        },
                        { "lineNumOld": 3, "lineNumNew": 4,
                          "line": "\n",
                          "type": "context"
                        }
                    ]
                }]
            },
            "eof" : eof,
        }]
    });
    assert_eq!(serde_json::to_value(&diff).unwrap(), json);
}
