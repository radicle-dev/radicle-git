// Copyright © 2022 The Radicle Git Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Unit tests for radicle_surf::file_system

#[cfg(test)]
mod path {
    use radicle_surf::file_system::unsound;

    #[test]
    fn split_last_root_and_foo() {
        let path = unsound::path::new("foo");
        assert_eq!(path.split_last(), (vec![], unsound::label::new("foo")));
    }

    #[test]
    fn split_last_same_labels() {
        // An interesting case for when first == last, but doesn't imply a singleton
        // Path.
        let path = unsound::path::new("foo/bar/foo");
        assert_eq!(
            path.split_last(),
            (
                vec![unsound::label::new("foo"), unsound::label::new("bar")],
                unsound::label::new("foo")
            )
        );
    }
}

#[cfg(test)]
mod directory {
    use git_ref_format::refname;
    use radicle_surf::{
        file_system::DirectoryEntry,
        git::{Branch, Repository},
    };
    use std::path::Path;

    const GIT_PLATINUM: &str = "../data/git-platinum";

    #[test]
    fn directory_get_path() {
        let repo = Repository::open(GIT_PLATINUM).unwrap();
        let root = repo.root_dir(&Branch::local(refname!("master"))).unwrap();

        // get_path for a file.
        let path = Path::new("src/memory.rs");
        let entry = root.get_path(path, &repo).unwrap();
        assert!(matches!(entry, DirectoryEntry::File(_)));

        // get_path for a directory.
        let path = Path::new("this/is/a/really/deeply/nested/directory/tree");
        let entry = root.get_path(path, &repo).unwrap();
        assert!(matches!(entry, DirectoryEntry::Directory(_)));

        // get_path for a non-leaf directory and its relative path.
        let path = Path::new("text");
        let entry = root.get_path(path, &repo).unwrap();
        assert!(matches!(entry, DirectoryEntry::Directory(_)));
        if let DirectoryEntry::Directory(sub_dir) = entry {
            let inner_path = Path::new("garden.txt");
            let inner_entry = sub_dir.get_path(inner_path, &repo).unwrap();
            assert!(matches!(inner_entry, DirectoryEntry::File(_)));
        }

        // get_path for non-existing file
        let path = Path::new("this/is/a/really/missing_file");
        let result = root.get_path(path, &repo);
        assert!(result.is_err());

        // get_path for absolute path: fail.
        let path = Path::new("/src/memory.rs");
        let result = root.get_path(path, &repo);
        assert!(result.is_err());
    }

    #[test]
    fn directory_size() {
        let repo = Repository::open(GIT_PLATINUM).unwrap();
        let root = repo.root_dir(&Branch::local(refname!("master"))).unwrap();

        /*
        git-platinum (master) $ ls -l src
        -rw-r--r-- 1 pi pi 10044 Oct 31 11:32 Eval.hs
        -rw-r--r-- 1 pi pi  6253 Oct 31 11:27 memory.rs

        10044 + 6253 = 16297
         */

        let path = Path::new("src");
        let entry = root.get_path(path, &repo).unwrap();
        assert!(matches!(entry, DirectoryEntry::Directory(_)));
        if let DirectoryEntry::Directory(d) = entry {
            assert_eq!(16297, d.size(&repo).unwrap());
        }
    }
}
