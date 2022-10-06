// Copyright © 2022 The Radicle Git Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Unit tests for radicle_surf::file_system

#[cfg(test)]
mod list_directory {
    use radicle_surf::file_system::{unsound, Directory, File, Label};

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

        let files: Vec<Label> = directory.contents().map(|c| c.label().clone()).collect();
        assert_eq!(
            files,
            vec![
                "bar.hs".parse::<Label>().unwrap(),
                "baz.hs".parse::<Label>().unwrap(),
                "foo.hs".parse::<Label>().unwrap(),
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
        directory.insert_file(unsound::path::new("bar/foo.hs"), file.clone());

        assert_eq!(
            directory.find_file(unsound::path::new("bar/foo.hs")),
            Some(&file)
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
