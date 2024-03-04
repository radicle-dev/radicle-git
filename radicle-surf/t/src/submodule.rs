#[test]
fn test_submodule() {
    use radicle_git_ext::ref_format::refname;
    use radicle_surf::{fs, Branch, Repository};

    let repo = Repository::discover(".").unwrap();
    let dir = repo
        .root_dir(Branch::local(refname!("surf/submodule-support")))
        .unwrap();
    let platinum = dir
        .find_entry(
            &std::path::Path::new("radicle-surf/data/git-platinum"),
            &repo,
        )
        .unwrap();
    assert!(matches!(platinum, fs::Entry::Submodule(_)));
}
