// Copyright © 2022 The Radicle Git Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Unit tests for radicle_surf::file_system

mod directory {
    use git_ref_format::refname;
    use radicle_surf::{
        file_system::{directory, Entry},
        git::{Branch, Repository},
    };
    use std::path::Path;

    const GIT_PLATINUM: &str = "../data/git-platinum";

    #[test]
    fn directory_find_entry() {
        let repo = Repository::open(GIT_PLATINUM).unwrap();
        let root = repo.root_dir(Branch::local(refname!("master"))).unwrap();

        // find_entry for a file.
        let path = Path::new("src/memory.rs");
        let entry = root.find_entry(&path, &repo).unwrap();
        assert!(matches!(entry, Some(directory::Entry::File(_))));

        // find_entry for a directory.
        let path = Path::new("this/is/a/really/deeply/nested/directory/tree");
        let entry = root.find_entry(&path, &repo).unwrap();
        assert!(matches!(entry, Some(directory::Entry::Directory(_))));

        // find_entry for a non-leaf directory and its relative path.
        let path = Path::new("text");
        let entry = root.find_entry(&path, &repo).unwrap();
        assert!(matches!(entry, Some(directory::Entry::Directory(_))));
        if let Some(directory::Entry::Directory(sub_dir)) = entry {
            let inner_path = Path::new("garden.txt");
            let inner_entry = sub_dir.find_entry(&inner_path, &repo).unwrap();
            assert!(matches!(inner_entry, Some(directory::Entry::File(_))));
        }

        // find_entry for non-existing file
        let path = Path::new("this/is/a/really/missing_file");
        let result = root.find_entry(&path, &repo).unwrap();
        assert_eq!(result, None);

        // find_entry for absolute path: fail.
        let path = Path::new("/src/memory.rs");
        let result = root.find_entry(&path, &repo);
        assert!(result.is_err());
    }

    #[test]
    fn directory_find_file_and_directory() {
        let repo = Repository::open(GIT_PLATINUM).unwrap();
        // Get the snapshot of the directory for a given commit.
        let root = repo
            .root_dir("80ded66281a4de2889cc07293a8f10947c6d57fe")
            .unwrap();

        // Assert that we can find the memory.rs file!
        assert!(root
            .find_file(&Path::new("src/memory.rs"), &repo)
            .unwrap()
            .is_some());

        let root_contents: Vec<Entry> = root.entries(&repo).unwrap().collect();
        assert_eq!(root_contents.len(), 7);
        assert!(root_contents[0].is_file());
        assert!(root_contents[1].is_file());
        assert!(root_contents[2].is_file());
        assert_eq!(root_contents[0].name(), ".i-am-well-hidden");
        assert_eq!(root_contents[1].name(), ".i-too-am-hidden");
        assert_eq!(root_contents[2].name(), "README.md");

        assert!(root_contents[3].is_directory());
        assert!(root_contents[4].is_directory());
        assert!(root_contents[5].is_directory());
        assert!(root_contents[6].is_directory());
        assert_eq!(root_contents[3].name(), "bin");
        assert_eq!(root_contents[4].name(), "src");
        assert_eq!(root_contents[5].name(), "text");
        assert_eq!(root_contents[6].name(), "this");

        let src = root
            .find_directory(&Path::new("src"), &repo)
            .unwrap()
            .unwrap();
        assert_eq!(src.path(), Path::new("src").to_path_buf());
        let src_contents: Vec<Entry> = src.entries(&repo).unwrap().collect();
        assert_eq!(src_contents.len(), 3);
        assert_eq!(src_contents[0].name(), "Eval.hs");
        assert_eq!(src_contents[1].name(), "Folder.svelte");
        assert_eq!(src_contents[2].name(), "memory.rs");
    }

    #[test]
    fn directory_size() {
        let repo = Repository::open(GIT_PLATINUM).unwrap();
        let root = repo.root_dir(Branch::local(refname!("master"))).unwrap();

        /*
        git-platinum (master) $ ls -l src
        -rw-r--r-- 1 pi pi 10044 Oct 31 11:32 Eval.hs
        -rw-r--r-- 1 pi pi  6253 Oct 31 11:27 memory.rs

        10044 + 6253 = 16297
         */

        let path = Path::new("src");
        let entry = root.find_entry(&path, &repo).unwrap();
        assert!(matches!(entry, Some(directory::Entry::Directory(_))));
        if let Some(directory::Entry::Directory(d)) = entry {
            assert_eq!(16297, d.size(&repo).unwrap());
        }
    }

    #[test]
    fn directory_last_commit() {
        let repo = Repository::open(GIT_PLATINUM).unwrap();
        let root = repo.root_dir(Branch::local(refname!("dev"))).unwrap();
        let dir = root.find_directory(&"this/is", &repo).unwrap().unwrap();
        let last_commit = dir.last_commit();
        assert_eq!(
            last_commit.id.to_string(),
            "2429f097664f9af0c5b7b389ab998b2199ffa977"
        );
    }
}
