use std::path::PathBuf;

use git_ref_format::refname;
use radicle_surf::{Glob, Repository};
use serde_json::json;

const GIT_PLATINUM: &str = "../data/git-platinum";

#[test]
fn tree_serialization() {
    let repo = Repository::open(GIT_PLATINUM).unwrap();
    let tree = repo.tree(refname!("refs/heads/master"), &"src").unwrap();

    let expected = json!({
      "entries": [
        {
          "kind": "blob",
          "lastCommit": {
            "author": {
              "email": "fintan.halpenny@gmail.com",
              "name": "Fintan Halpenny",
              "time": 1578309972
            },
            "committer": {
              "email": "noreply@github.com",
              "name": "GitHub",
              "time": 1578309972
            },
            "description": "I want to have files under src that have separate commits.\r\nThat way src's latest commit isn't the same as all its files, instead it's the file that was touched last.",
            "id": "3873745c8f6ffb45c990eb23b491d4b4b6182f95",
            "message": "Extend the docs (#2)\n\nI want to have files under src that have separate commits.\r\nThat way src's latest commit isn't the same as all its files, instead it's the file that was touched last.",
            "parents": ["d6880352fc7fda8f521ae9b7357668b17bb5bad5"],
            "summary": "Extend the docs (#2)"
          },
          "name": "Eval.hs",
          "oid": "7d6240123a8d8ea8a8376610168a0a4bcb96afd0"
        },
        {
          "kind": "blob",
          "lastCommit": {
            "author": {
              "email": "rudolfs@osins.org",
              "name": "Rūdolfs Ošiņš",
              "time": 1575283266
            },
            "committer": {
              "email": "rudolfs@osins.org",
              "name": "Rūdolfs Ošiņš",
              "time": 1575283266
            },
            "description": "",
            "id": "e24124b7538658220b5aaf3b6ef53758f0a106dc",
            "message": "Move examples to \"src\"\n",
            "parents": ["19bec071db6474af89c866a1bd0e4b1ff76e2b97"],
            "summary": "Move examples to \"src\""
          },
          "name": "memory.rs",
          "oid": "b84992d24be67536837f5ab45a943f1b3f501878"
        }
      ],
      "lastCommit": {
        "author": {
          "email": "rudolfs@osins.org",
          "name": "Rūdolfs Ošiņš",
          "time": 1582198877
        },
        "committer": {
          "email": "noreply@github.com",
          "name": "GitHub",
          "time": 1582198877
        },
        "description": "It was a bad idea to have an actual source file which is used by\r\nradicle-upstream in the fixtures repository. It gets in the way of\r\nlinting and editors pick it up as a regular source file by accident.",
        "id": "a57846bbc8ced6587bf8329fc4bce970eb7b757e",
        "message": "Remove src/Folder.svelte (#3)\n\nIt was a bad idea to have an actual source file which is used by\r\nradicle-upstream in the fixtures repository. It gets in the way of\r\nlinting and editors pick it up as a regular source file by accident.",
        "parents": ["3873745c8f6ffb45c990eb23b491d4b4b6182f95"],
        "summary": "Remove src/Folder.svelte (#3)"
      },
      "oid": "ed52e9f8dfe1d8b374b2a118c25235349a743dd2"
    });
    assert_eq!(serde_json::to_value(tree).unwrap(), expected)
}

#[test]
fn repo_tree() {
    let repo = Repository::open(GIT_PLATINUM).unwrap();
    let tree = repo
        .tree("27acd68c7504755aa11023300890bb85bbd69d45", &"src")
        .unwrap();
    assert_eq!(tree.entries().len(), 3);

    let commit_header = tree.commit();
    assert_eq!(
        commit_header.id.to_string(),
        "e24124b7538658220b5aaf3b6ef53758f0a106dc"
    );

    let tree_oid = tree.object_id();
    assert_eq!(
        tree_oid.to_string(),
        "dbd5d80c64a00969f521b96401a315e9481e9561"
    );

    let entries = tree.entries();
    assert_eq!(entries.len(), 3);
    let entry = &entries[0];
    assert!(!entry.is_tree());
    assert_eq!(entry.name(), "Eval.hs");
    assert_eq!(
        entry.object_id().to_string(),
        "8c7447d13b907aa994ac3a38317c1e9633bf0732"
    );
    let commit = entry.commit();
    assert_eq!(
        commit.id.to_string(),
        "e24124b7538658220b5aaf3b6ef53758f0a106dc"
    );

    // Verify that an empty path works for getting the root tree.
    let root_tree = repo
        .tree("27acd68c7504755aa11023300890bb85bbd69d45", &"")
        .unwrap();
    assert_eq!(root_tree.entries().len(), 8);
}

#[test]
fn repo_blob() {
    let repo = Repository::open(GIT_PLATINUM).unwrap();
    let blob = repo
        .blob("27acd68c7504755aa11023300890bb85bbd69d45", &"src/memory.rs")
        .unwrap();

    let blob_oid = blob.object_id();
    assert_eq!(
        blob_oid.to_string(),
        "b84992d24be67536837f5ab45a943f1b3f501878"
    );

    let commit_header = blob.commit();
    assert_eq!(
        commit_header.id.to_string(),
        "e24124b7538658220b5aaf3b6ef53758f0a106dc"
    );

    assert!(!blob.is_binary());

    // Verify the blob content size matches with the file size of "memory.rs"
    let content = blob.content();
    assert_eq!(content.size(), 6253);

    // Verify as_bytes.
    assert_eq!(content.as_bytes().len(), content.size());
}

#[test]
fn tree_ordering() {
    let repo = Repository::open(GIT_PLATINUM).unwrap();
    let tree = repo
        .tree(refname!("refs/heads/master"), &PathBuf::new())
        .unwrap();
    assert_eq!(
        tree.entries()
            .iter()
            .map(|entry| entry.name().to_string())
            .collect::<Vec<_>>(),
        vec![
            "bin".to_string(),
            "special".to_string(),
            "src".to_string(),
            "text".to_string(),
            "this".to_string(),
            ".i-am-well-hidden".to_string(),
            ".i-too-am-hidden".to_string(),
            "README.md".to_string(),
        ]
    );
}

#[test]
fn commit_branches() {
    let repo = Repository::open(GIT_PLATINUM).unwrap();
    let init_commit = "d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3";
    let glob = Glob::all_heads().branches().and(Glob::all_remotes());
    let branches = repo.revision_branches(init_commit, glob).unwrap();

    assert_eq!(branches.len(), 7);
    assert_eq!(branches[0].refname().as_str(), "refs/heads/dev");
    assert_eq!(branches[1].refname().as_str(), "refs/heads/master");
    assert_eq!(
        branches[2].refname().as_str(),
        "refs/remotes/banana/orange/pineapple"
    );
    assert_eq!(
        branches[3].refname().as_str(),
        "refs/remotes/banana/pineapple"
    );
    assert_eq!(branches[4].refname().as_str(), "refs/remotes/origin/HEAD");
    assert_eq!(branches[5].refname().as_str(), "refs/remotes/origin/dev");
    assert_eq!(branches[6].refname().as_str(), "refs/remotes/origin/master");
}
