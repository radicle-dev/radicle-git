use std::path::Path;

use git_ref_format::refname;
use radicle_surf::{git::Repository, source};
use serde_json::json;

const GIT_PLATINUM: &str = "../data/git-platinum";

#[test]
fn tree_serialization() {
    let repo = Repository::open(GIT_PLATINUM).unwrap();
    let tree = source::Tree::new(
        &repo,
        &refname!("refs/heads/master"),
        Some(&Path::new("src")),
    )
    .unwrap();

    let expected = json!({
      "entries": [
        {
          "kind": "blob",
          "lastCommit": null,
          "name": "Eval.hs",
          "path": "src/Eval.hs"
        },
        {
          "kind": "blob",
          "lastCommit": null,
          "name": "memory.rs",
          "path": "src/memory.rs"
        }
      ],
      "lastCommit": null,
      "name": "src",
      "path": "src"
    });
    assert_eq!(serde_json::to_value(tree).unwrap(), expected)
}
