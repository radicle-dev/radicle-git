// Copyright © 2022 The Radicle Git Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Unit tests for radicle_surf::vcs::git and its submodules.

#[cfg(feature = "serialize")]
use radicle_surf::git::{Author, BranchType, Commit};
use radicle_surf::{
    diff::*,
    file_system::{unsound, DirectoryContents, Path},
    git::{error::Error, Branch, BranchName, Namespace, Oid, RefScope, Repository, Rev, TagName},
};

const GIT_PLATINUM: &str = "../data/git-platinum";

#[cfg(not(feature = "gh-actions"))]
#[test]
// An issue with submodules, see: https://github.com/radicle-dev/radicle-surf/issues/54
fn test_submodule_failure() {
    let repo = Repository::new("../..").unwrap();
    repo.as_ref()
        .snapshot(&Branch::local("main").into())
        .unwrap();
}

#[cfg(test)]
mod namespace {
    use super::*;
    use pretty_assertions::{assert_eq, assert_ne};
    use radicle_surf::vcs::Vcs;

    #[test]
    fn switch_to_banana() -> Result<(), Error> {
        let repo = Repository::new(GIT_PLATINUM)?;
        let repo = repo.as_ref();
        let history_master = repo.get_history(Branch::local("master").into())?;
        repo.switch_namespace("golden")?;
        let history_banana = repo.get_history(Branch::local("banana").into())?;

        assert_ne!(history_master, history_banana);

        Ok(())
    }

    #[test]
    fn me_namespace() -> Result<(), Error> {
        let repo = Repository::new(GIT_PLATINUM)?;
        let repo = repo.as_ref();
        let history = repo.get_history(Branch::local("master").into())?;

        assert_eq!(repo.which_namespace(), Ok(None));

        repo.switch_namespace("me")?;
        assert_eq!(repo.which_namespace(), Ok(Some(Namespace::try_from("me")?)));

        let history_feature = repo.get_history(Branch::local("feature/#1194").into())?;
        assert_eq!(history, history_feature);

        let expected_branches: Vec<Branch> = vec![Branch::local("feature/#1194")];
        let mut branches = repo.list_branches(RefScope::Local)?;
        branches.sort();

        assert_eq!(expected_branches, branches);

        let expected_branches: Vec<Branch> = vec![Branch::remote("feature/#1194", "fein")];
        let mut branches = repo.list_branches(RefScope::Remote {
            name: Some("fein".to_string()),
        })?;
        branches.sort();

        assert_eq!(expected_branches, branches);

        Ok(())
    }

    #[test]
    fn golden_namespace() -> Result<(), Error> {
        let repo = Repository::new(GIT_PLATINUM)?;
        let repo = repo.as_ref();
        let history = repo.get_history(Branch::local("master").into())?;

        assert_eq!(repo.which_namespace(), Ok(None));

        repo.switch_namespace("golden")?;

        assert_eq!(
            repo.which_namespace(),
            Ok(Some(Namespace::try_from("golden")?))
        );

        let golden_history = repo.get_history(Branch::local("master").into())?;
        assert_eq!(history, golden_history);

        let expected_branches: Vec<Branch> = vec![Branch::local("banana"), Branch::local("master")];
        let mut branches = repo.list_branches(RefScope::Local)?;
        branches.sort();

        assert_eq!(expected_branches, branches);

        let expected_branches: Vec<Branch> = vec![
            Branch::remote("fakie/bigspin", "kickflip"),
            Branch::remote("heelflip", "kickflip"),
            Branch::remote("v0.1.0", "kickflip"),
        ];
        let mut branches = repo.list_branches(RefScope::Remote {
            name: Some("kickflip".to_string()),
        })?;
        branches.sort();

        assert_eq!(expected_branches, branches);

        Ok(())
    }

    #[test]
    fn silver_namespace() -> Result<(), Error> {
        let repo = Repository::new(GIT_PLATINUM)?;
        let repo = repo.as_ref();
        let history = repo.get_history(Branch::local("master").into())?;

        assert_eq!(repo.which_namespace(), Ok(None));

        repo.switch_namespace("golden/silver")?;
        assert_eq!(
            repo.which_namespace(),
            Ok(Some(Namespace::try_from("golden/silver")?))
        );
        let silver_history = repo.get_history(Branch::local("master").into())?;
        assert_ne!(history, silver_history);

        let expected_branches: Vec<Branch> = vec![Branch::local("master")];
        let mut branches = repo.list_branches(RefScope::All)?;
        branches.sort();

        assert_eq!(expected_branches, branches);

        Ok(())
    }
}

#[cfg(test)]
mod rev {
    use radicle_surf::vcs::Vcs;

    use super::*;
    use std::str::FromStr;

    // **FIXME**: This seems to break occasionally on
    // buildkite. For some reason the commit
    // 3873745c8f6ffb45c990eb23b491d4b4b6182f95, which is on master
    // (currently HEAD), is not found. It seems to load the history
    // with d6880352fc7fda8f521ae9b7357668b17bb5bad5 as the HEAD.
    //
    // To temporarily fix this, we need to select "New Build" from the build kite
    // build page that's failing.
    // * Under "Message" put whatever you want.
    // * Under "Branch" put in the branch you're working on.
    // * Expand "Options" and select "clean checkout".
    #[test]
    fn _master() -> Result<(), Error> {
        let repo = Repository::new(GIT_PLATINUM)?;
        let repo = repo.as_ref();
        let history = repo.get_history(Branch::remote("master", "origin").into())?;

        let commit1 = Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?;
        assert!(
            history
                .find(|commit| if commit.id == commit1 {
                    Some(commit.clone())
                } else {
                    None
                })
                .is_some(),
            "commit_id={}, history =\n{:#?}",
            commit1,
            &history
        );

        let commit2 = Oid::from_str("d6880352fc7fda8f521ae9b7357668b17bb5bad5")?;
        assert!(
            history
                .find(|commit| if commit.id == commit2 {
                    Some(commit.clone())
                } else {
                    None
                })
                .is_some(),
            "commit_id={}, history =\n{:#?}",
            commit2,
            &history
        );

        Ok(())
    }

    #[test]
    fn commit() -> Result<(), Error> {
        let repo = Repository::new(GIT_PLATINUM)?;
        let rev: Rev = Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?.into();
        let history = repo.as_ref().get_history(rev)?;

        let commit1 = Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?;
        assert!(history
            .find(|commit| if commit.id == commit1 {
                Some(commit.clone())
            } else {
                None
            })
            .is_some());

        Ok(())
    }

    #[test]
    fn commit_parents() -> Result<(), Error> {
        let repo = Repository::new(GIT_PLATINUM)?;
        let rev: Rev = Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?.into();
        let history = repo.as_ref().get_history(rev)?;
        let commit = history.first();

        assert_eq!(
            commit.parents,
            vec![Oid::from_str("d6880352fc7fda8f521ae9b7357668b17bb5bad5")?]
        );

        Ok(())
    }

    #[test]
    fn commit_short() -> Result<(), Error> {
        let repo = Repository::new(GIT_PLATINUM)?;
        let rev: Rev = repo.as_ref().oid("3873745c8")?.into();
        let history = repo.as_ref().get_history(rev)?;

        let commit1 = Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?;
        assert!(history
            .find(|commit| if commit.id == commit1 {
                Some(commit.clone())
            } else {
                None
            })
            .is_some());

        Ok(())
    }

    #[test]
    fn tag() -> Result<(), Error> {
        let repo = Repository::new(GIT_PLATINUM)?;
        let rev: Rev = TagName::new("v0.2.0").into();
        let history = repo.as_ref().get_history(rev)?;

        let commit1 = Oid::from_str("2429f097664f9af0c5b7b389ab998b2199ffa977")?;
        assert_eq!(history.first().id, commit1);

        Ok(())
    }
}

#[cfg(test)]
mod last_commit {
    use radicle_surf::vcs::Vcs;

    use super::*;
    use std::str::FromStr;

    #[test]
    fn readme_missing_and_memory() {
        let repo = Repository::new(GIT_PLATINUM)
            .expect("Could not retrieve ./data/git-platinum as git repository");
        let oid =
            Oid::from_str("d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3").expect("Failed to parse SHA");

        // memory.rs is commited later so it should not exist here.
        let rev: Rev = oid.into();
        let memory_last_commit_oid = repo
            .as_ref()
            .last_commit(
                Path::with_root(&[unsound::label::new("src"), unsound::label::new("memory.rs")]),
                &rev,
            )
            .expect("Failed to get last commit")
            .map(|commit| commit.id);

        assert_eq!(memory_last_commit_oid, None);

        // README.md exists in this commit.
        let readme_last_commit = repo
            .as_ref()
            .last_commit(Path::with_root(&[unsound::label::new("README.md")]), &rev)
            .expect("Failed to get last commit")
            .map(|commit| commit.id);

        assert_eq!(readme_last_commit, Some(oid));
    }

    #[test]
    fn folder_svelte() {
        let repo = Repository::new(GIT_PLATINUM)
            .expect("Could not retrieve ./data/git-platinum as git repository");
        // Check that last commit is the actual last commit even if head commit differs.
        let oid =
            Oid::from_str("19bec071db6474af89c866a1bd0e4b1ff76e2b97").expect("Could not parse SHA");
        let rev: Rev = oid.into();

        let expected_commit_id = Oid::from_str("f3a089488f4cfd1a240a9c01b3fcc4c34a4e97b2").unwrap();

        let folder_svelte = repo
            .as_ref()
            .last_commit(unsound::path::new("~/examples/Folder.svelte"), &rev)
            .expect("Failed to get last commit")
            .map(|commit| commit.id);

        assert_eq!(folder_svelte, Some(expected_commit_id));
    }

    #[test]
    fn nest_directory() {
        let repo = Repository::new(GIT_PLATINUM)
            .expect("Could not retrieve ./data/git-platinum as git repository");
        // Check that last commit is the actual last commit even if head commit differs.
        let oid =
            Oid::from_str("19bec071db6474af89c866a1bd0e4b1ff76e2b97").expect("Failed to parse SHA");
        let rev: Rev = oid.into();

        let expected_commit_id = Oid::from_str("2429f097664f9af0c5b7b389ab998b2199ffa977").unwrap();

        let nested_directory_tree_commit_id = repo
            .as_ref()
            .last_commit(
                unsound::path::new("~/this/is/a/really/deeply/nested/directory/tree"),
                &rev,
            )
            .expect("Failed to get last commit")
            .map(|commit| commit.id);

        assert_eq!(nested_directory_tree_commit_id, Some(expected_commit_id));
    }

    #[test]
    #[cfg(not(windows))]
    fn can_get_last_commit_for_special_filenames() {
        let repo = Repository::new(GIT_PLATINUM)
            .expect("Could not retrieve ./data/git-platinum as git repository");

        // Check that last commit is the actual last commit even if head commit differs.
        let oid =
            Oid::from_str("a0dd9122d33dff2a35f564d564db127152c88e02").expect("Failed to parse SHA");
        let rev: Rev = oid.into();

        let expected_commit_id = Oid::from_str("a0dd9122d33dff2a35f564d564db127152c88e02").unwrap();

        let backslash_commit_id = repo
            .as_ref()
            .last_commit(unsound::path::new("~/special/faux\\path"), &rev)
            .expect("Failed to get last commit")
            .map(|commit| commit.id);
        assert_eq!(backslash_commit_id, Some(expected_commit_id));

        let ogre_commit_id = repo
            .as_ref()
            .last_commit(unsound::path::new("~/special/👹👹👹"), &rev)
            .expect("Failed to get last commit")
            .map(|commit| commit.id);
        assert_eq!(ogre_commit_id, Some(expected_commit_id));
    }

    #[test]
    fn root() {
        let repo = Repository::new(GIT_PLATINUM)
            .expect("Could not retrieve ./data/git-platinum as git repository");
        let rev: Rev = Branch::local("master").into();
        let root_last_commit_id = repo
            .as_ref()
            .last_commit(Path::root(), &rev)
            .expect("Failed to get last commit")
            .map(|commit| commit.id);

        let expected_oid = repo
            .as_ref()
            .get_history(Branch::local("master").into())
            .unwrap()
            .first()
            .id;
        assert_eq!(root_last_commit_id, Some(expected_oid));
    }
}

#[cfg(test)]
mod diff {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::str::FromStr;

    #[test]
    fn test_initial_diff() -> Result<(), Error> {
        let oid = Oid::from_str("d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3")?;
        let repo = Repository::new(GIT_PLATINUM)?;
        let repo = repo.as_ref();
        let commit = repo.get_git2_commit(oid).unwrap();

        assert!(commit.parents().count() == 0);
        assert!(commit.parent(0).is_err());

        let diff = repo.initial_diff(&oid.into())?;

        let expected_diff = Diff {
            created: vec![CreateFile {
                path: Path::with_root(&[unsound::label::new("README.md")]),
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
    fn test_diff() -> Result<(), Error> {
        let repo = Repository::new(GIT_PLATINUM)?;
        let repo = repo.as_ref();
        let oid = Oid::from_str("80bacafba303bf0cdf6142921f430ff265f25095")?;
        let commit = repo.get_git2_commit(oid).unwrap();
        let parent = commit.parent(0)?;
        let parent_oid: Oid = parent.id().into();
        let diff = repo.diff(&parent_oid.into(), &oid.into())?;

        let expected_diff = Diff {
                created: vec![],
                deleted: vec![],
                moved: vec![],
                copied: vec![],
                modified: vec![ModifiedFile {
                    path: Path::with_root(&[unsound::label::new("README.md")]),
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

    #[cfg(feature = "serialize")]
    #[test]
    fn test_diff_serde() -> Result<(), Error> {
        let diff = Diff {
                created: vec![CreateFile{path: unsound::path::new("LICENSE"), diff: FileDiff::Plain { hunks: Hunks::default() }}],
                deleted: vec![],
                moved: vec![
                    MoveFile {
                        old_path: unsound::path::new("CONTRIBUTING"),
                        new_path: unsound::path::new("CONTRIBUTING.md")
                    }
                ],
                copied: vec![],
                modified: vec![ModifiedFile {
                    path: Path::with_root(&[unsound::label::new("README.md")]),
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

        Ok(())
    }
}

#[cfg(test)]
mod threading {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    #[test]
    fn basic_test() -> Result<(), Error> {
        let shared_repo = Mutex::new(Repository::new(GIT_PLATINUM)?);
        let locked_repo: MutexGuard<Repository> = shared_repo.lock().unwrap();
        let mut branches = locked_repo.as_ref().list_branches(RefScope::All)?;
        branches.sort();

        assert_eq!(
            branches,
            vec![
                Branch::remote("HEAD", "origin"),
                Branch::remote("dev", "origin"),
                Branch::local("dev"),
                Branch::remote("master", "origin"),
                Branch::local("master"),
                Branch::remote("orange/pineapple", "banana"),
                Branch::remote("pineapple", "banana"),
            ]
        );

        Ok(())
    }
}

#[cfg(feature = "serialize")]
#[cfg(test)]
mod commit {
    use super::{Author, Commit};
    use proptest::prelude::*;
    use radicle_git_ext::Oid;
    use std::str::FromStr;
    use test_helpers::roundtrip;

    #[cfg(feature = "serialize")]
    proptest! {
        #[test]
        fn prop_test_commits(commit in commits_strategy()) {
            roundtrip::json(commit)
        }
    }

    fn commits_strategy() -> impl Strategy<Value = Commit> {
        ("[a-fA-F0-9]{40}", any::<String>(), any::<i64>()).prop_map(|(id, text, time)| Commit {
            id: Oid::from_str(&id).unwrap(),
            author: Author {
                name: text.clone(),
                email: text.clone(),
                time: git2::Time::new(time, 0),
            },
            committer: Author {
                name: text.clone(),
                email: text.clone(),
                time: git2::Time::new(time, 0),
            },
            message: text.clone(),
            summary: text,
            parents: vec![Oid::from_str(&id).unwrap(), Oid::from_str(&id).unwrap()],
        })
    }
}

#[cfg(feature = "serialize")]
#[cfg(test)]
mod branch {
    use super::*;
    use proptest::prelude::*;
    use test_helpers::roundtrip;

    proptest! {
        #[test]
        fn prop_test_branch(branch in branch_strategy()) {
            roundtrip::json(branch)
        }
    }

    fn branch_strategy() -> impl Strategy<Value = Branch> {
        prop_oneof![
            any::<String>().prop_map(|name| Branch {
                name: BranchName::new(&name),
                locality: BranchType::Local
            }),
            (any::<String>(), any::<String>()).prop_map(|(name, remote_name)| Branch {
                name: BranchName::new(&name),
                locality: BranchType::Remote {
                    name: Some(remote_name),
                },
            })
        ]
    }
}

#[cfg(test)]
mod ext {
    use radicle_surf::vcs::git::ext::*;

    #[test]
    fn test_try_extract_refname() {
        assert_eq!(try_extract_refname("refs/heads/dev"), Ok("dev".to_string()));

        assert_eq!(
            try_extract_refname("refs/heads/master"),
            Ok("master".to_string())
        );

        assert_eq!(
            try_extract_refname("refs/remotes/banana/pineapple"),
            Ok("banana/pineapple".to_string())
        );

        assert_eq!(
            try_extract_refname("refs/remotes/origin/master"),
            Ok("origin/master".to_string())
        );

        assert_eq!(
            try_extract_refname("refs/namespaces/golden/refs/heads/banana"),
            Ok("banana".to_string())
        );

        assert_eq!(
            try_extract_refname("refs/namespaces/golden/refs/tags/v0.1.0"),
            Ok("v0.1.0".to_string())
        );

        assert_eq!(
            try_extract_refname("refs/namespaces/golden/refs/namespaces/silver/refs/heads/master"),
            Ok("master".to_string())
        );

        assert_eq!(
            try_extract_refname("refs/namespaces/golden/refs/remotes/kickflip/heads/heelflip"),
            Ok("kickflip/heelflip".to_string())
        );
    }
}

#[cfg(test)]
mod reference {
    use super::*;
    use radicle_surf::vcs::git::{ParseError, Ref};
    use std::str::FromStr;

    #[test]
    fn parse_ref() -> Result<(), ParseError> {
        assert_eq!(
            Ref::from_str("refs/remotes/origin/master"),
            Ok(Ref::RemoteBranch {
                remote: "origin".to_string(),
                name: BranchName::new("master")
            })
        );

        assert_eq!(
            Ref::from_str("refs/heads/master"),
            Ok(Ref::LocalBranch {
                name: BranchName::new("master"),
            })
        );

        assert_eq!(
            Ref::from_str("refs/heads/i-am-hyphenated"),
            Ok(Ref::LocalBranch {
                name: BranchName::new("i-am-hyphenated"),
            })
        );

        assert_eq!(
            Ref::from_str("refs/heads/prefix/i-am-hyphenated"),
            Ok(Ref::LocalBranch {
                name: BranchName::new("prefix/i-am-hyphenated"),
            })
        );

        assert_eq!(
            Ref::from_str("refs/tags/v0.0.1"),
            Ok(Ref::Tag {
                name: TagName::new("v0.0.1")
            })
        );

        assert_eq!(
            Ref::from_str("refs/namespaces/moi/refs/remotes/origin/master"),
            Ok(Ref::Namespace {
                namespace: "moi".to_string(),
                reference: Box::new(Ref::RemoteBranch {
                    remote: "origin".to_string(),
                    name: BranchName::new("master")
                })
            })
        );

        assert_eq!(
            Ref::from_str("refs/namespaces/moi/refs/namespaces/toi/refs/tags/v1.0.0"),
            Ok(Ref::Namespace {
                namespace: "moi".to_string(),
                reference: Box::new(Ref::Namespace {
                    namespace: "toi".to_string(),
                    reference: Box::new(Ref::Tag {
                        name: TagName::new("v1.0.0")
                    })
                })
            })
        );

        assert_eq!(
            Ref::from_str("refs/namespaces/me/refs/heads/feature/#1194"),
            Ok(Ref::Namespace {
                namespace: "me".to_string(),
                reference: Box::new(Ref::LocalBranch {
                    name: BranchName::new("feature/#1194"),
                })
            })
        );

        assert_eq!(
            Ref::from_str("refs/namespaces/me/refs/remotes/fein/heads/feature/#1194"),
            Ok(Ref::Namespace {
                namespace: "me".to_string(),
                reference: Box::new(Ref::RemoteBranch {
                    remote: "fein".to_string(),
                    name: BranchName::new("heads/feature/#1194"),
                })
            })
        );

        assert_eq!(
            Ref::from_str("refs/remotes/master"),
            Err(ParseError::MalformedRef("refs/remotes/master".to_owned())),
        );

        assert_eq!(
            Ref::from_str("refs/namespaces/refs/remotes/origin/master"),
            Err(ParseError::MalformedRef(
                "refs/namespaces/refs/remotes/origin/master".to_owned()
            )),
        );

        Ok(())
    }
}

mod code_browsing {
    use super::*;
    use radicle_surf::file_system::Directory;

    #[test]
    fn iterate_root_dir_recursive() {
        let repo = Repository::new(GIT_PLATINUM).unwrap();
        let repo = repo.as_ref();
        let root_dir = repo.snapshot(&Branch::local("master").into()).unwrap();
        let count = println_dir(&root_dir, 0);
        assert_eq!(count, 36); // Check total file count.

        /// Prints items in `dir` with `indent_level`.
        /// For sub-directories, will do Depth-First-Search and print
        /// recursively.
        /// Returns the number of items visited (i.e. printed)
        fn println_dir(dir: &Directory, indent_level: usize) -> i32 {
            let mut count = 0;
            for item in dir.contents() {
                println!("> {}{}", " ".repeat(indent_level * 4), &item.label());
                count += 1;
                if let DirectoryContents::Directory(sub_dir) = item {
                    count += println_dir(sub_dir, indent_level + 1);
                }
            }
            count
        }
    }
}
