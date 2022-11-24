use radicle_surf::git::{Glob, Repository};

use super::GIT_PLATINUM;

#[test]
fn test_branches() {
    let repo = Repository::open(GIT_PLATINUM).unwrap();
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
    let tags = repo
        .tags(&Glob::tags("*").unwrap())
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(tags.len(), 6);
    let root_dir = repo.root_dir(&tags[0]).unwrap();
    assert_eq!(root_dir.entries(&repo).unwrap().entries().count(), 1);
}

#[test]
fn test_namespaces() {
    let repo = Repository::open(GIT_PLATINUM).unwrap();
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
