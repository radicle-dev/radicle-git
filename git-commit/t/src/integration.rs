use std::io;

use git_commit::Commit;
use test_helpers::tempdir::WithTmpDir;

#[test]
fn valid_commits() {
    let radicle_git = format!(
        "file://{}",
        git2::Repository::discover(".").unwrap().path().display()
    );
    let repo = WithTmpDir::new(|path| {
        let repo = git2::Repository::clone(&radicle_git, path)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        Ok::<_, io::Error>(repo)
    })
    .unwrap();

    let mut walk = repo.revwalk().unwrap();
    walk.push_head().unwrap();

    // take the first 20 commits and make sure we can parse them
    for oid in walk.take(20) {
        let oid = oid.unwrap();
        let commit = Commit::read(&repo, oid);
        assert!(commit.is_ok(), "Oid: {}, Error: {:?}", oid, commit)
    }
}
