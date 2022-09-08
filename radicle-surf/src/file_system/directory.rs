// This file is part of radicle-surf
// <https://github.com/radicle-dev/radicle-surf>
//
// Copyright (C) 2019-2020 The Radicle Team <dev@radicle.xyz>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 or
// later as published by the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Definition for a file system consisting of `Directory` and `File`.
//!
//! A `Directory` is expected to be a non-empty tree of directories and files.
//! See [`Directory`] for more information.
//!
//! As well as this, this module contains [`DirectoryContents`] which is the
//! output of iterating over a [`Directory`], and also [`SystemType`] which is
//! an identifier of what type of [`DirectoryContents`] one is viewing when
//! [listing](#method.list_directory) a directory.

use crate::{file_system::path::*, tree::*};
use nonempty::NonEmpty;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

/// `SystemType` is an enumeration over what can be found in a [`Directory`] so
/// we can report back to the caller a [`Label`] and its type.
///
/// See [`SystemType::file`](#method.file) and
/// [`SystemType::directory`](#method.directory).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SystemType {
    /// The `File` type in a directory system.
    File,
    /// The `Directory` type in a directory system.
    Directory,
}

impl SystemType {
    /// A file name and [`SystemType::File`].
    pub fn file(label: Label) -> (Label, Self) {
        (label, SystemType::File)
    }

    /// A directory name and [`SystemType::Directory`].
    pub fn directory(label: Label) -> (Label, Self) {
        (label, SystemType::Directory)
    }
}

/// A `File` consists of its file contents (a [`Vec`] of bytes).
///
/// The `Debug` instance of `File` will show the first few bytes of the file and
/// its [`size`](#method.size).
#[derive(Clone, PartialEq, Eq)]
pub struct File {
    /// The contents of a `File` as a vector of bytes.
    pub contents: Vec<u8>,
    pub(crate) size: usize,
}

impl std::fmt::Debug for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut contents = self.contents.clone();
        contents.truncate(10);
        write!(
            f,
            "File {{ contents: {:?}, size: {} }}",
            contents, self.size
        )
    }
}

impl File {
    /// Create a new `File` with the contents provided.
    pub fn new(contents: &[u8]) -> Self {
        let size = contents.len();
        File {
            contents: contents.to_vec(),
            size,
        }
    }

    /// Get the size of the `File` corresponding to the number of bytes in the
    /// file contents.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::File;
    ///
    /// let file = File::new(b"pub mod diff;\npub mod file_system;\npub mod vcs;\npub use crate::vcs::git;\n");
    ///
    /// assert_eq!(file.size(), 73);
    /// ```
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get the hash of the `File` corresponding to the contents of the file.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::File;
    ///
    /// let file = File::new(
    ///     b"pub mod diff;\npub mod file_system;\npub mod vcs;\npub use crate::vcs::git;\n",
    /// );
    ///
    /// assert_eq!(file.checksum(), 8457766712413557403);
    /// ```
    pub fn checksum(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.contents.hash(&mut hasher);
        hasher.finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Location {
    Root,
    SubDirectory(Label),
}

/// A `Directory` can be thought of as a non-empty set of entries of
/// sub-directories and files. The reason for the non-empty property is that a
/// VCS directory would have at least one artifact as a sub-directory which
/// tracks the VCS work, e.g. git using the `.git` folder.
///
/// On top of that, some VCSes, such as git, will not track an empty directory,
/// and so when creating a new directory to track it will have to contain at
/// least one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directory {
    current: Location,
    sub_directories: Forest<Label, File>,
}

/// `DirectoryContents` is an enumeration of what a [`Directory`] can contain
/// and is used for when we are [`iter`](struct.Directory.html#method.iter)ating
/// through a [`Directory`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectoryContents {
    /// The `File` variant contains the file's name and the [`File`] itself.
    File {
        /// The name of the file.
        name: Label,
        /// The file data.
        file: File,
    },
    /// The `Directory` variant contains a sub-directory to the current one.
    Directory(Directory),
}

impl DirectoryContents {
    /// Get a label for the `DirectoryContents`, either the name of the [`File`]
    /// or the name of the [`Directory`].
    pub fn label(&self) -> Label {
        match self {
            DirectoryContents::File { name, .. } => name.clone(),
            DirectoryContents::Directory(directory) => directory.current(),
        }
    }
}

impl From<SubTree<Label, File>> for DirectoryContents {
    fn from(sub_tree: SubTree<Label, File>) -> Self {
        match sub_tree {
            SubTree::Node { key, value } => DirectoryContents::File {
                name: key,
                file: value,
            },
            SubTree::Branch { key, forest } => DirectoryContents::Directory(Directory {
                current: Location::SubDirectory(key),
                sub_directories: (*forest).into(),
            }),
        }
    }
}

impl Directory {
    /// Create a root directory.
    ///
    /// This function is usually used for testing and demonstation purposes.
    pub fn root() -> Self {
        Directory {
            current: Location::Root,
            sub_directories: Forest::root(),
        }
    }

    /// Create a directory, similar to `root`, except with a given name.
    ///
    /// This function is usually used for testing and demonstation purposes.
    pub fn new(label: Label) -> Self {
        Directory {
            current: Location::SubDirectory(label),
            sub_directories: Forest::root(),
        }
    }

    /// List the current `Directory`'s files and sub-directories.
    ///
    /// The listings are a pair of [`Label`] and [`SystemType`], where the
    /// [`Label`] represents the name of the file or directory.
    ///
    /// ```
    /// use nonempty::NonEmpty;
    /// use radicle_surf::file_system::{Directory, File, SystemType};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let mut directory = Directory::root();
    ///
    /// // Root files set up
    /// let root_files = NonEmpty::from((
    ///     (unsound::label::new("foo.rs"), File::new(b"use crate::bar")),
    ///     vec![(
    ///         unsound::label::new("bar.rs"),
    ///         File::new(b"fn hello_world()"),
    ///     )],
    /// ));
    /// directory.insert_files(&[], root_files);
    ///
    /// // Haskell files set up
    /// let haskell_files = NonEmpty::from((
    ///     (
    ///         unsound::label::new("foo.hs"),
    ///         File::new(b"module Foo where"),
    ///     ),
    ///     vec![(
    ///         unsound::label::new("bar.hs"),
    ///         File::new(b"module Bar where"),
    ///     )],
    /// ));
    ///
    /// directory.insert_files(&[unsound::label::new("haskell")], haskell_files);
    ///
    /// let mut directory_contents = directory.list_directory();
    /// directory_contents.sort();
    ///
    /// assert_eq!(
    ///     directory_contents,
    ///     vec![
    ///         SystemType::file(unsound::label::new("bar.rs")),
    ///         SystemType::file(unsound::label::new("foo.rs")),
    ///         SystemType::directory(unsound::label::new("haskell")),
    ///     ]
    /// );
    /// ```
    pub fn list_directory(&self) -> Vec<(Label, SystemType)> {
        let forest = &self.sub_directories;
        match &forest.0 {
            None => vec![],
            Some(trees) => trees
                .0
                .iter()
                .map(|tree| match tree {
                    SubTree::Node { key: name, .. } => SystemType::file(name.clone()),
                    SubTree::Branch { key: name, .. } => SystemType::directory(name.clone()),
                })
                .collect(),
        }
    }

    /// Get the [`Label`] of the current directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::{Directory, DirectoryContents, File, Label};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let mut root = Directory::root();
    ///
    /// let main = File::new(b"println!(\"Hello, world!\")");
    /// root.insert_file(unsound::path::new("main.rs"), main.clone());
    ///
    /// let lib = File::new(b"struct Hello(String)");
    /// root.insert_file(unsound::path::new("lib.rs"), lib.clone());
    ///
    /// let test_mod = File::new(b"assert_eq!(1 + 1, 2);");
    /// root.insert_file(unsound::path::new("test/mod.rs"), test_mod.clone());
    ///
    /// let mut root_iter = root.iter();
    ///
    /// assert_eq!(root_iter.next(), Some(DirectoryContents::File {
    ///     name: unsound::label::new("lib.rs"),
    ///     file: lib
    /// }));
    ///
    /// assert_eq!(root_iter.next(), Some(DirectoryContents::File {
    ///     name: unsound::label::new("main.rs"),
    ///     file: main
    /// }));
    ///
    /// let mut test_dir = Directory::new(unsound::label::new("test"));
    /// test_dir.insert_file(unsound::path::new("mod.rs"), test_mod);
    ///
    /// assert_eq!(root_iter.next(), Some(DirectoryContents::Directory(test_dir)));
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = DirectoryContents> + '_ {
        let mut empty_iter = None;
        let mut trees_iter = None;
        match &self.sub_directories.0 {
            None => empty_iter = Some(std::iter::empty()),
            Some(trees) => {
                trees_iter = Some(
                    trees
                        .iter_subtrees()
                        .cloned()
                        .map(|sub_tree| sub_tree.into()),
                )
            },
        }

        empty_iter
            .into_iter()
            .flatten()
            .chain(trees_iter.into_iter().flatten())
    }

    /// Find a [`File`] in the directory given the [`Path`] to the [`File`].
    ///
    /// # Failures
    ///
    /// This operation fails if the path does not lead to a [`File`].
    /// If the search is for a `Directory` then use `find_directory`.
    ///
    /// # Examples
    ///
    /// Search for a file in the path:
    ///     * `foo/bar/baz.hs`
    ///     * `foo`
    ///     * `foo/bar/qux.rs`
    ///
    /// ```
    /// use radicle_surf::file_system::{Directory, File};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let file = File::new(b"module Banana ...");
    ///
    /// let mut directory = Directory::root();
    /// directory.insert_file(unsound::path::new("foo/bar/baz.rs"), file.clone());
    ///
    /// // The file is succesfully found
    /// assert_eq!(directory.find_file(unsound::path::new("foo/bar/baz.rs")), Some(file));
    ///
    /// // We shouldn't be able to find a directory
    /// assert_eq!(directory.find_file(unsound::path::new("foo")), None);
    ///
    /// // We shouldn't be able to find a file that doesn't exist
    /// assert_eq!(directory.find_file(unsound::path::new("foo/bar/qux.rs")), None);
    /// ```
    pub fn find_file(&self, path: Path) -> Option<File> {
        self.sub_directories.find_node(path.0).cloned()
    }

    /// Find a `Directory` in the directory given the [`Path`] to the
    /// `Directory`.
    ///
    /// # Failures
    ///
    /// This operation fails if the path does not lead to the `Directory`.
    ///
    /// # Examples
    ///
    /// Search for directories in the path:
    ///     * `foo`
    ///     * `foo/bar`
    ///     * `foo/baz`
    ///
    /// ```
    /// use radicle_surf::file_system::{Directory, File};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let file = File::new(b"module Banana ...");
    ///
    /// let mut directory = Directory::root();
    /// directory.insert_file(unsound::path::new("foo/bar/baz.rs"), file.clone());
    ///
    /// // Can find the first level
    /// assert!(directory.find_directory(unsound::path::new("foo")).is_some());
    ///
    /// // Can find the second level
    /// assert!(directory.find_directory(unsound::path::new("foo/bar")).is_some());
    ///
    /// // Cannot find 'baz' since it does not exist
    /// assert!(directory.find_directory(unsound::path::new("foo/baz")).is_none());
    ///
    /// // 'baz.rs' is a file and not a directory
    /// assert!(directory.find_directory(unsound::path::new("foo/bar/baz.rs")).is_none());
    /// ```
    pub fn find_directory(&self, path: Path) -> Option<Self> {
        self.sub_directories
            .find_branch(path.0.clone())
            .cloned()
            .map(|tree| {
                let (_, current) = path.split_last();
                Directory {
                    current: Location::SubDirectory(current),
                    sub_directories: tree.into(),
                }
            })
    }

    /// Get the [`Label`] of the current directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::{Directory, File, Label};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let mut root = Directory::root();
    /// root.insert_file(unsound::path::new("main.rs"), File::new(b"println!(\"Hello, world!\")"));
    /// root.insert_file(unsound::path::new("lib.rs"), File::new(b"struct Hello(String)"));
    /// root.insert_file(unsound::path::new("test/mod.rs"), File::new(b"assert_eq!(1 + 1, 2);"));
    ///
    /// assert_eq!(root.current(), Label::root());
    ///
    /// let test = root.find_directory(
    ///     unsound::path::new("test")
    /// ).expect("Missing test directory");
    /// assert_eq!(test.current(), unsound::label::new("test"));
    /// ```
    pub fn current(&self) -> Label {
        match &self.current {
            Location::Root => Label::root(),
            Location::SubDirectory(label) => label.clone(),
        }
    }

    // TODO(fintan): This is going to be a bit trickier so going to leave it out for
    // now
    #[allow(dead_code)]
    fn fuzzy_find(_label: Label) -> Vec<Self> {
        unimplemented!()
    }

    /// Get the total size, in bytes, of a `Directory`. The size is
    /// the sum of all files that can be reached from this `Directory`.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::{Directory, File};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let mut root = Directory::root();
    /// root.insert_file(unsound::path::new("main.rs"), File::new(b"println!(\"Hello, world!\")"));
    /// root.insert_file(unsound::path::new("lib.rs"), File::new(b"struct Hello(String)"));
    /// root.insert_file(unsound::path::new("test/mod.rs"), File::new(b"assert_eq!(1 + 1, 2);"));
    ///
    /// assert_eq!(root.size(), 66);
    /// ```
    pub fn size(&self) -> usize {
        self.sub_directories
            .iter()
            .fold(0, |size, file| size + file.size())
    }

    /// Insert a file into a directory, given the full path to file (file name
    /// inclusive) and the `File` itself.
    ///
    /// This function is usually used for testing and demonstation purposes.
    pub fn insert_file(&mut self, path: Path, file: File) {
        self.sub_directories.insert(path.0, file)
    }

    /// Insert files into a shared directory path.
    ///
    /// `directory_path` is used as the prefix to where the files should go. If
    /// empty the files will be placed in the current `Directory`.
    ///
    /// `files` are pairs of file name and the [`File`] itself.
    ///
    /// This function is usually used for testing and demonstation purposes.
    pub fn insert_files(&mut self, directory_path: &[Label], files: NonEmpty<(Label, File)>) {
        match NonEmpty::from_slice(directory_path) {
            None => {
                for (file_name, file) in files.into_iter() {
                    self.insert_file(Path::new(file_name), file)
                }
            },
            Some(path) => {
                for (file_name, file) in files.into_iter() {
                    // The clone is necessary here because we use it as a prefix.
                    let mut file_path = Path(path.clone());
                    file_path.push(file_name);

                    self.insert_file(file_path, file)
                }
            },
        }
    }

    pub(crate) fn from_hash_map(files: HashMap<Path, NonEmpty<(Label, File)>>) -> Self {
        let mut directory: Self = Directory::root();

        for (path, files) in files.into_iter() {
            for (file_name, file) in files.into_iter() {
                let file_path = if path.is_root() {
                    Path::new(file_name)
                } else {
                    let mut new_path = path.clone();
                    new_path.push(file_name);
                    new_path
                };

                directory.insert_file(file_path, file)
            }
        }

        directory
    }
}

#[cfg(test)]
pub mod tests {
    #[cfg(test)]
    mod list_directory {
        use crate::file_system::{unsound, Directory, File, SystemType};

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
        use crate::file_system::{unsound, *};

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
        use crate::file_system::{unsound, Directory, File};
        use nonempty::NonEmpty;

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
        use crate::file_system::{unsound, *};
        use nonempty::NonEmpty;
        use proptest::{collection, prelude::*};
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
}
