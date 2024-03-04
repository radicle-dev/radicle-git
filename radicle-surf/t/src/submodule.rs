use std::path::Path;

use radicle_surf::tree::EntryKind;

#[test]
fn test_submodule() {
    use radicle_git_ext::ref_format::refname;
    use radicle_surf::{fs, Branch, Repository};

    let repo = Repository::discover(".").unwrap();
    let branch = Branch::local(refname!("surf/submodule-support"));
    let dir = repo.root_dir(&branch).unwrap();
    let platinum = dir
        .find_entry(&Path::new("radicle-surf/data/git-platinum"), &repo)
        .unwrap();
    assert!(matches!(&platinum, fs::Entry::Submodule(module) if module.url().is_some()));

    let surf = repo.tree(&branch, &Path::new("radicle-surf/data")).unwrap();
    let kind = EntryKind::from(platinum);
    assert!(surf.entries().iter().any(|e| e.entry() == &kind));
}
