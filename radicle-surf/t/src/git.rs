// Copyright © 2022 The Radicle Git Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Unit tests for radicle_surf::git and its submodules.

#[cfg(feature = "serialize")]
use radicle_surf::git::{Author, Commit};
use radicle_surf::{
    diff::*,
    file_system::{unsound, DirectoryEntry, Path},
    git::{Branch, Error, Glob, Namespace, Oid, Repository},
};

const GIT_PLATINUM: &str = "../data/git-platinum";

#[cfg(not(feature = "gh-actions"))]
#[test]
// An issue with submodules, see: https://github.com/radicle-dev/radicle-surf/issues/54
fn test_submodule_failure() {
    use git_ref_format::refname;

    let repo = Repository::discover(".").unwrap();
    repo.as_ref()
        .root_dir(&Branch::local(refname!("main")))
        .unwrap();
}

#[cfg(test)]
mod namespace {
    use super::*;
    use git_ref_format::{name::component, refname};
    use pretty_assertions::{assert_eq, assert_ne};

    #[test]
    fn switch_to_banana() -> Result<(), Error> {
        let repo = Repository::open(GIT_PLATINUM)?;
        let repo = repo.as_ref();
        let history_master = repo.history(&Branch::local(refname!("master")))?;
        repo.switch_namespace("golden")?;
        let history_banana = repo.history(&Branch::local(refname!("banana")))?;

        assert_ne!(history_master.head(), history_banana.head());

        Ok(())
    }

    #[test]
    fn me_namespace() -> Result<(), Error> {
        let repo = Repository::open(GIT_PLATINUM)?;
        let repo = repo.as_ref();
        let history = repo.history(&Branch::local(refname!("master")))?;

        assert_eq!(repo.which_namespace().unwrap(), None);

        repo.switch_namespace("me")?;
        assert_eq!(
            repo.which_namespace().unwrap(),
            Some(Namespace::try_from("me")?)
        );

        let history_feature = repo.history(&Branch::local(refname!("feature/#1194")))?;
        assert_eq!(history.head(), history_feature.head());

        let expected_branches: Vec<Branch> = vec![Branch::local(refname!("feature/#1194"))];
        let mut branches = repo
            .branches(&Glob::heads("*")?)?
            .collect::<Result<Vec<_>, _>>()?;
        branches.sort();

        assert_eq!(expected_branches, branches);

        let expected_branches: Vec<Branch> = vec![Branch::remote(
            component!("fein"),
            refname!("heads/feature/#1194"),
        )];
        let mut branches = repo
            .branches(&Glob::remotes("fein/*")?)?
            .collect::<Result<Vec<_>, _>>()?;
        branches.sort();

        assert_eq!(expected_branches, branches);

        Ok(())
    }

    #[test]
    fn golden_namespace() -> Result<(), Error> {
        let repo = Repository::open(GIT_PLATINUM)?;
        let repo = repo.as_ref();
        let history = repo.history(&Branch::local(refname!("master")))?;

        assert_eq!(repo.which_namespace().unwrap(), None);

        repo.switch_namespace("golden")?;

        assert_eq!(
            repo.which_namespace().unwrap(),
            Some(Namespace::try_from("golden")?)
        );

        let golden_history = repo.history(&Branch::local(refname!("master")))?;
        assert_eq!(history.head(), golden_history.head());

        let expected_branches: Vec<Branch> = vec![
            Branch::local(refname!("banana")),
            Branch::local(refname!("master")),
        ];
        let mut branches = repo
            .branches(&Glob::heads("*")?)?
            .collect::<Result<Vec<_>, _>>()?;
        branches.sort();

        assert_eq!(expected_branches, branches);

        // NOTE: these tests used to remove the categories, i.e. heads & tags, but that
        // was specialised logic based on the radicle-link storage layout.
        let remote = component!("kickflip");
        let expected_branches: Vec<Branch> = vec![
            Branch::remote(remote.clone(), refname!("heads/fakie/bigspin")),
            Branch::remote(remote.clone(), refname!("heads/heelflip")),
            Branch::remote(remote, refname!("tags/v0.1.0")),
        ];
        let mut branches = repo
            .branches(&Glob::remotes("kickflip/*")?)?
            .collect::<Result<Vec<_>, _>>()?;
        branches.sort();

        assert_eq!(expected_branches, branches);

        Ok(())
    }

    #[test]
    fn silver_namespace() -> Result<(), Error> {
        let repo = Repository::open(GIT_PLATINUM)?;
        let repo = repo.as_ref();
        let history = repo.history(&Branch::local(refname!("master")))?;

        assert_eq!(repo.which_namespace().unwrap(), None);

        repo.switch_namespace("golden/silver")?;
        assert_eq!(
            repo.which_namespace().unwrap(),
            Some(Namespace::try_from("golden/silver")?)
        );
        let silver_history = repo.history(&Branch::local(refname!("master")))?;
        assert_ne!(history.head(), silver_history.head());

        let expected_branches: Vec<Branch> = vec![Branch::local(refname!("master"))];
        let mut branches = repo
            .branches(&Glob::heads("*")?.and_remotes("*")?)?
            .collect::<Result<Vec<_>, _>>()?;
        branches.sort();

        assert_eq!(expected_branches, branches);

        Ok(())
    }
}

#[cfg(test)]
mod rev {
    use git_ref_format::{name::component, refname};

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
        let repo = Repository::open(GIT_PLATINUM)?;
        let repo = repo.as_ref();
        let mut history =
            repo.history(&Branch::remote(component!("origin"), refname!("master")))?;

        let commit1 = Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?;
        assert!(
            history.any(|commit| commit.unwrap().id == commit1),
            "commit_id={}, history =\n{:#?}",
            commit1,
            &history
        );

        let commit2 = Oid::from_str("d6880352fc7fda8f521ae9b7357668b17bb5bad5")?;
        assert!(
            history.any(|commit| commit.unwrap().id == commit2),
            "commit_id={}, history =\n{:#?}",
            commit2,
            &history
        );

        Ok(())
    }

    #[test]
    fn commit() -> Result<(), Error> {
        let repo = Repository::open(GIT_PLATINUM)?;
        let repo = repo.as_ref();
        let rev = Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?;
        let mut history = repo.history(rev)?;

        let commit1 = Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?;
        assert!(history.any(|commit| commit.unwrap().id == commit1));

        Ok(())
    }

    #[test]
    fn commit_parents() -> Result<(), Error> {
        let repo = Repository::open(GIT_PLATINUM)?;
        let repo = repo.as_ref();
        let rev = Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?;
        let history = repo.history(rev)?;
        let commit = history.head();

        assert_eq!(
            commit.parents,
            vec![Oid::from_str("d6880352fc7fda8f521ae9b7357668b17bb5bad5")?]
        );

        Ok(())
    }

    #[test]
    fn commit_short() -> Result<(), Error> {
        let repo = Repository::open(GIT_PLATINUM)?;
        let repo = repo.as_ref();
        let rev = repo.oid("3873745c8")?;
        let mut history = repo.history(rev)?;

        let commit1 = Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?;
        assert!(history.any(|commit| commit.unwrap().id == commit1));

        Ok(())
    }

    #[test]
    fn tag() -> Result<(), Error> {
        let repo = Repository::open(GIT_PLATINUM)?;
        let repo = repo.as_ref();
        let rev = refname!("refs/tags/v0.2.0");
        let history = repo.history(&rev)?;

        let commit1 = Oid::from_str("2429f097664f9af0c5b7b389ab998b2199ffa977")?;
        assert_eq!(history.head().id, commit1);

        Ok(())
    }
}

#[cfg(test)]
mod last_commit {
    use git_ref_format::refname;

    use super::*;
    use std::str::FromStr;

    #[test]
    fn readme_missing_and_memory() {
        let repo = Repository::open(GIT_PLATINUM)
            .expect("Could not retrieve ./data/git-platinum as git repository");
        let oid =
            Oid::from_str("d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3").expect("Failed to parse SHA");

        // memory.rs is commited later so it should not exist here.
        let memory_last_commit_oid = repo
            .as_ref()
            .last_commit(
                Path::with_root(&[unsound::label::new("src"), unsound::label::new("memory.rs")]),
                oid,
            )
            .expect("Failed to get last commit")
            .map(|commit| commit.id);

        assert_eq!(memory_last_commit_oid, None);

        // README.md exists in this commit.
        let readme_last_commit = repo
            .as_ref()
            .last_commit(Path::with_root(&[unsound::label::new("README.md")]), oid)
            .expect("Failed to get last commit")
            .map(|commit| commit.id);

        assert_eq!(readme_last_commit, Some(oid));
    }

    #[test]
    fn folder_svelte() {
        let repo = Repository::open(GIT_PLATINUM)
            .expect("Could not retrieve ./data/git-platinum as git repository");
        // Check that last commit is the actual last commit even if head commit differs.
        let oid =
            Oid::from_str("19bec071db6474af89c866a1bd0e4b1ff76e2b97").expect("Could not parse SHA");

        let expected_commit_id = Oid::from_str("f3a089488f4cfd1a240a9c01b3fcc4c34a4e97b2").unwrap();

        let folder_svelte = repo
            .as_ref()
            .last_commit(unsound::path::new("~/examples/Folder.svelte"), oid)
            .expect("Failed to get last commit")
            .map(|commit| commit.id);

        assert_eq!(folder_svelte, Some(expected_commit_id));
    }

    #[test]
    fn nest_directory() {
        let repo = Repository::open(GIT_PLATINUM)
            .expect("Could not retrieve ./data/git-platinum as git repository");
        // Check that last commit is the actual last commit even if head commit differs.
        let oid =
            Oid::from_str("19bec071db6474af89c866a1bd0e4b1ff76e2b97").expect("Failed to parse SHA");

        let expected_commit_id = Oid::from_str("2429f097664f9af0c5b7b389ab998b2199ffa977").unwrap();

        let nested_directory_tree_commit_id = repo
            .as_ref()
            .last_commit(
                unsound::path::new("~/this/is/a/really/deeply/nested/directory/tree"),
                oid,
            )
            .expect("Failed to get last commit")
            .map(|commit| commit.id);

        assert_eq!(nested_directory_tree_commit_id, Some(expected_commit_id));
    }

    #[test]
    #[cfg(not(windows))]
    fn can_get_last_commit_for_special_filenames() {
        let repo = Repository::open(GIT_PLATINUM)
            .expect("Could not retrieve ./data/git-platinum as git repository");

        // Check that last commit is the actual last commit even if head commit differs.
        let oid =
            Oid::from_str("a0dd9122d33dff2a35f564d564db127152c88e02").expect("Failed to parse SHA");

        let expected_commit_id = Oid::from_str("a0dd9122d33dff2a35f564d564db127152c88e02").unwrap();

        let backslash_commit_id = repo
            .as_ref()
            .last_commit(unsound::path::new("~/special/faux\\path"), oid)
            .expect("Failed to get last commit")
            .map(|commit| commit.id);
        assert_eq!(backslash_commit_id, Some(expected_commit_id));

        let ogre_commit_id = repo
            .as_ref()
            .last_commit(unsound::path::new("~/special/👹👹👹"), oid)
            .expect("Failed to get last commit")
            .map(|commit| commit.id);
        assert_eq!(ogre_commit_id, Some(expected_commit_id));
    }

    #[test]
    fn root() {
        let repo = Repository::open(GIT_PLATINUM)
            .expect("Could not retrieve ./data/git-platinum as git repository");
        let rev = Branch::local(refname!("master"));
        let root_last_commit_id = repo
            .as_ref()
            .last_commit(Path::root(), &rev)
            .expect("Failed to get last commit")
            .map(|commit| commit.id);

        let expected_oid = repo
            .as_ref()
            .history(&Branch::local(refname!("master")))
            .unwrap()
            .head()
            .id;
        assert_eq!(root_last_commit_id, Some(expected_oid));
    }

    #[test]
    fn binary_file() {
        let repo = Repository::open(GIT_PLATINUM)
            .expect("Could not retrieve ./data/git-platinum as git repository");
        let repo = repo.as_ref();
        let history = repo.history(&Branch::local(refname!("dev"))).unwrap();
        let file_commit = history.by_path(unsound::path::new("~/bin/cat")).next();
        assert!(file_commit.is_some());
        println!("file commit: {:?}", &file_commit);
    }
}

#[cfg(test)]
mod diff {
    use super::*;
    use git_ref_format::refname;
    use pretty_assertions::assert_eq;
    use std::str::FromStr;

    #[test]
    fn test_initial_diff() -> Result<(), Error> {
        let repo = Repository::open(GIT_PLATINUM)?;
        let repo = repo.as_ref();
        let oid = Oid::from_str("d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3")?;
        let commit = repo.commit(oid).unwrap();
        assert!(commit.parents.is_empty());

        let diff = repo.initial_diff(oid)?;

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
    fn test_diff_of_rev() -> Result<(), Error> {
        let repo = Repository::open(GIT_PLATINUM)?;
        let repo = repo.as_ref();
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
        let repo = repo.as_ref();
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

    #[test]
    fn test_branch_diff() -> Result<(), Error> {
        let repo = Repository::open(GIT_PLATINUM)?;
        let repo = repo.as_ref();
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
            println!("created: {}", &c.path);
        }
        for d in diff.deleted.iter() {
            println!("deleted: {}", &d.path);
        }
        for m in diff.moved.iter() {
            println!("moved: {} -> {}", &m.old_path, &m.new_path);
        }
        for m in diff.modified.iter() {
            println!("modified: {}", &m.path);
        }
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
    use git_ref_format::{name::component, refname};
    use radicle_surf::git::Glob;

    use super::*;
    use std::sync::{Mutex, MutexGuard};

    #[test]
    fn basic_test() -> Result<(), Error> {
        let shared_repo = Mutex::new(Repository::open(GIT_PLATINUM)?);
        let locked_repo: MutexGuard<Repository> = shared_repo.lock().unwrap();
        let mut branches = locked_repo
            .as_ref()
            .branches(&Glob::heads("*")?.and_remotes("*")?)?
            .collect::<Result<Vec<_>, _>>()?;
        branches.sort();

        let origin = component!("origin");
        let banana = component!("banana");
        assert_eq!(
            branches,
            vec![
                Branch::local(refname!("dev")),
                Branch::local(refname!("master")),
                Branch::remote(banana.clone(), refname!("orange/pineapple")),
                Branch::remote(banana, refname!("pineapple")),
                Branch::remote(origin.clone(), refname!("HEAD")),
                Branch::remote(origin.clone(), refname!("dev")),
                Branch::remote(origin, refname!("master")),
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
    use git_ref_format::{RefStr, RefString};
    use git_ref_format_test::gen;
    use proptest::prelude::*;
    use test_helpers::roundtrip;

    use super::*;

    proptest! {
        #[test]
        fn prop_test_branch(branch in gen_branch()) {
            roundtrip::json(branch)
        }
    }

    fn gen_branch() -> impl Strategy<Value = Branch> {
        prop_oneof![
            gen::valid().prop_map(|name| Branch::local(RefString::try_from(name).unwrap())),
            (gen::valid(), gen::valid()).prop_map(|(remote, name): (String, String)| {
                let remote =
                    RefStr::try_from_str(&remote).expect("BUG: reference strings should be valid");
                let name =
                    RefStr::try_from_str(&name).expect("BUG: reference strings should be valid");
                Branch::remote(remote.head(), name)
            })
        ]
    }
}

#[cfg(test)]
mod reference {
    use super::*;
    use radicle_surf::git::Glob;

    #[test]
    fn test_branches() {
        let repo = Repository::open(GIT_PLATINUM).unwrap();
        let repo = repo.as_ref();
        let branches = repo.branches(&Glob::heads("*").unwrap()).unwrap();
        for b in branches {
            println!("{}", b.unwrap().refname());
        }
        let branches = repo
            .branches(&Glob::heads("*").unwrap().and_remotes("banana/*").unwrap())
            .unwrap();
        for b in branches {
            println!("{}", b.unwrap().refname());
        }
    }

    #[test]
    fn test_tag_snapshot() {
        let repo = Repository::open(GIT_PLATINUM).unwrap();
        let repo_ref = repo.as_ref();
        let tags = repo_ref
            .tags(&Glob::tags("*").unwrap())
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(tags.len(), 6);
        let root_dir = repo_ref.root_dir(&tags[0]).unwrap();
        assert_eq!(root_dir.contents(&repo_ref).unwrap().iter().count(), 1);
    }

    #[test]
    fn test_namespaces() {
        let repo = Repository::open(GIT_PLATINUM).unwrap();
        let repo = repo.as_ref();
        let namespaces = repo.namespaces(&Glob::namespaces("*").unwrap()).unwrap();
        assert_eq!(namespaces.count(), 3);
        let namespaces = repo
            .namespaces(&Glob::namespaces("golden/*").unwrap())
            .unwrap();
        assert_eq!(namespaces.count(), 2);
        let namespaces = repo
            .namespaces(&Glob::namespaces("golden/*").unwrap().and("me/*").unwrap())
            .unwrap();
        assert_eq!(namespaces.count(), 3);
    }
}

mod code_browsing {
    use super::*;

    use git_ref_format::refname;
    use radicle_surf::{file_system::Directory, git::RepositoryRef};

    #[test]
    fn iterate_root_dir_recursive() {
        let repo = Repository::open(GIT_PLATINUM).unwrap();
        let repo = repo.as_ref();

        let root_dir = repo.root_dir(&Branch::local(refname!("master"))).unwrap();
        let count = println_dir(&root_dir, &repo, 0);

        assert_eq!(count, 36); // Check total file count.

        /// Prints items in `dir` with `indent_level`.
        /// For sub-directories, will do Depth-First-Search and print
        /// recursively.
        /// Returns the number of items visited (i.e. printed)
        fn println_dir(dir: &Directory, repo: &RepositoryRef, indent_level: usize) -> i32 {
            let mut count = 0;
            for item in dir.contents(repo).unwrap().iter() {
                println!("> {}{}", " ".repeat(indent_level * 4), &item.label());
                count += 1;
                if let DirectoryEntry::Directory(sub_dir) = item {
                    count += println_dir(sub_dir, repo, indent_level + 1);
                }
            }
            count
        }
    }

    #[test]
    fn browse_repo_lazily() {
        let repo = Repository::open(GIT_PLATINUM).unwrap();
        let repo = repo.as_ref();
        let root_dir = repo.root_dir(&Branch::local(refname!("master"))).unwrap();
        let count = root_dir.contents(&repo).unwrap().iter().count();
        assert_eq!(count, 8);
        let count = traverse(&root_dir, &repo);
        assert_eq!(count, 36);

        fn traverse(dir: &Directory, repo: &RepositoryRef) -> i32 {
            let mut count = 0;
            for item in dir.contents(repo).unwrap().iter() {
                count += 1;
                if let DirectoryEntry::Directory(sub_dir) = item {
                    count += traverse(sub_dir, repo)
                }
            }
            count
        }
    }

    #[test]
    fn test_file_history() {
        let repo = Repository::open(GIT_PLATINUM).unwrap();
        let repo = repo.as_ref();
        let history = repo.history(&Branch::local(refname!("dev"))).unwrap();
        let path = unsound::path::new("README.md");
        let mut file_history = history.by_path(path);
        let commit = file_history.next().unwrap().unwrap();
        let file = repo
            .get_commit_file(&commit.id, unsound::path::new("README.md"))
            .unwrap();
        assert_eq!(file.size(), 67);
    }
}
