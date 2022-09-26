// Copyright © 2022 The Radicle Git Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Unit tests for radicle_surf::file_system

#[cfg(test)]
mod list_directory {
    use radicle_surf::file_system::{unsound, Directory, File, SystemType};

    #[test]
    fn root_files() {
        let mut directory = Directory::root();
        directory.insert_file(
            unsound::path::new("foo.hs"),
            File::new(b"module BananaFoo ..."),
        );
        directory.insert_file(
            unsound::path::new("bar.hs"),
            File::new(b"module BananaBar ..."),
        );
        directory.insert_file(
            unsound::path::new("baz.hs"),
            File::new(b"module BananaBaz ..."),
        );

        assert_eq!(
            directory.list_directory(),
            vec![
                SystemType::file(unsound::label::new("bar.hs")),
                SystemType::file(unsound::label::new("baz.hs")),
                SystemType::file(unsound::label::new("foo.hs")),
            ]
        );
    }
}

#[cfg(test)]
mod find_file {
    use radicle_surf::file_system::{unsound, *};

    #[test]
    fn in_root() {
        let file = File::new(b"module Banana ...");
        let mut directory = Directory::root();
        directory.insert_file(unsound::path::new("foo.hs"), file.clone());

        assert_eq!(
            directory.find_file(unsound::path::new("foo.hs")),
            Some(file)
        );
    }

    #[test]
    fn file_does_not_exist() {
        let file_path = unsound::path::new("bar.hs");

        let file = File::new(b"module Banana ...");

        let mut directory = Directory::root();
        directory.insert_file(unsound::path::new("foo.hs"), file);

        assert_eq!(directory.find_file(file_path), None)
    }
}

#[cfg(test)]
mod directory_size {
    use nonempty::NonEmpty;
    use radicle_surf::file_system::{unsound, Directory, File};

    #[test]
    fn root_directory_files() {
        let mut root = Directory::root();
        root.insert_files(
            &[],
            NonEmpty::from((
                (
                    unsound::label::new("main.rs"),
                    File::new(b"println!(\"Hello, world!\")"),
                ),
                vec![(
                    unsound::label::new("lib.rs"),
                    File::new(b"struct Hello(String)"),
                )],
            )),
        );

        assert_eq!(root.size(), 45);
    }
}

#[cfg(test)]
mod properties {
    use nonempty::NonEmpty;
    use proptest::{collection, prelude::*};
    use radicle_surf::file_system::{unsound, *};
    use std::collections::HashMap;

    #[test]
    fn test_all_directories_and_files() {
        let mut directory_map = HashMap::new();

        let path1 = unsound::path::new("foo/bar/baz");
        let file1 = (unsound::label::new("monadic.rs"), File::new(&[]));
        let file2 = (unsound::label::new("oscoin.rs"), File::new(&[]));
        directory_map.insert(path1, NonEmpty::from((file1, vec![file2])));

        let path2 = unsound::path::new("foor/bar/quuz");
        let file3 = (unsound::label::new("radicle.rs"), File::new(&[]));

        directory_map.insert(path2, NonEmpty::new(file3));

        assert!(prop_all_directories_and_files(directory_map))
    }

    fn label_strategy() -> impl Strategy<Value = Label> {
        // ASCII regex, excluding '/' because of posix file paths
        "[ -.|0-~]+".prop_map(|label| unsound::label::new(&label))
    }

    fn path_strategy(max_size: usize) -> impl Strategy<Value = Path> {
        (
            label_strategy(),
            collection::vec(label_strategy(), 0..max_size),
        )
            .prop_map(|(label, labels)| Path((label, labels).into()))
    }

    fn file_strategy() -> impl Strategy<Value = (Label, File)> {
        // ASCII regex, see: https://catonmat.net/my-favorite-regex
        (label_strategy(), "[ -~]*")
            .prop_map(|(name, contents)| (name, File::new(contents.as_bytes())))
    }

    fn directory_map_strategy(
        path_size: usize,
        n_files: usize,
        map_size: usize,
    ) -> impl Strategy<Value = HashMap<Path, NonEmpty<(Label, File)>>> {
        collection::hash_map(
            path_strategy(path_size),
            collection::vec(file_strategy(), 1..n_files).prop_map(|files| {
                NonEmpty::from_slice(&files).expect("Strategy generated files of length 0")
            }),
            0..map_size,
        )
    }

    // TODO(fintan): This is a bit slow. Could be time to benchmark some functions.
    proptest! {
        #[test]
        fn prop_test_all_directories_and_files(directory_map in directory_map_strategy(10, 10, 10)) {
            prop_all_directories_and_files(directory_map);
        }
    }

    fn prop_all_directories_and_files(
        directory_map: HashMap<Path, NonEmpty<(Label, File)>>,
    ) -> bool {
        let mut new_directory_map = HashMap::new();
        for (path, files) in directory_map {
            new_directory_map.insert(path.clone(), files);
        }

        let directory = Directory::from_hash_map(new_directory_map.clone());

        for (directory_path, files) in new_directory_map {
            for (file_name, _) in files.iter() {
                let mut path = directory_path.clone();
                if directory.find_directory(path.clone()).is_none() {
                    eprintln!("Search Directory: {:#?}", directory);
                    eprintln!("Path to find: {:#?}", path);
                    return false;
                }

                path.push(file_name.clone());
                if directory.find_file(path.clone()).is_none() {
                    eprintln!("Search Directory: {:#?}", directory);
                    eprintln!("Path to find: {:#?}", path);
                    return false;
                }
            }
        }
        true
    }

    #[test]
    fn test_file_name_is_same_as_root() {
        // This test ensures that if the name is the same the root of the
        // directory, that search_path.split_last() doesn't toss away the prefix.
        let path = unsound::path::new("foo/bar/~");
        let mut directory_map = HashMap::new();
        directory_map.insert(path, NonEmpty::new((Label::root(), File::new(b"root"))));

        assert!(prop_all_directories_and_files(directory_map));
    }
}

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
