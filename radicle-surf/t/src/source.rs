use std::path::PathBuf;

use git_ref_format::refname;
use radicle_surf::Repository;
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
              "name": "Fintan Halpenny"
            },
            "committer": {
              "email": "noreply@github.com",
              "name": "GitHub"
            },
            "committerTime": 1578309972,
            "description": "I want to have files under src that have separate commits.\r\nThat way src's latest commit isn't the same as all its files, instead it's the file that was touched last.",
            "sha1": "3873745c8f6ffb45c990eb23b491d4b4b6182f95",
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
              "name": "Rūdolfs Ošiņš"
            },
            "committer": {
              "email": "rudolfs@osins.org",
              "name": "Rūdolfs Ošiņš"
            },
            "committerTime": 1575283266,
            "description": "",
            "sha1": "e24124b7538658220b5aaf3b6ef53758f0a106dc",
            "summary": "Move examples to \"src\""
          },
          "name": "memory.rs",
          "oid": "b84992d24be67536837f5ab45a943f1b3f501878"
        }
      ],
      "lastCommit": {
        "author": {
          "email": "rudolfs@osins.org",
          "name": "Rūdolfs Ošiņš"
        },
        "committer": {
          "email": "noreply@github.com",
          "name": "GitHub"
        },
        "committerTime": 1582198877,
        "description": "It was a bad idea to have an actual source file which is used by\r\nradicle-upstream in the fixtures repository. It gets in the way of\r\nlinting and editors pick it up as a regular source file by accident.",
        "sha1": "a57846bbc8ced6587bf8329fc4bce970eb7b757e",
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
        commit_header.sha1.to_string(),
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
        commit.sha1.to_string(),
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
        commit_header.sha1.to_string(),
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
