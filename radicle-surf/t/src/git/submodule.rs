#[cfg(not(feature = "gh-actions"))]
#[test]
// An issue with submodules, see: https://github.com/radicle-dev/radicle-surf/issues/54
fn test_submodule_failure() {
    use git_ref_format::refname;
    use radicle_surf::git::{Branch, Repository};

    let repo = Repository::discover(".").unwrap();
    repo.root_dir(&Branch::local(refname!("main"))).unwrap();
}
