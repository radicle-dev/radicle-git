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
//! output of iterating over a [`Directory`].

use crate::file_system::path::*;
use nonempty::NonEmpty;
use std::{
    collections::{hash_map::DefaultHasher, BTreeMap},
    hash::{Hash, Hasher},
};

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

/// A `Directory` is a set of entries of sub-directories and files, ordered
/// by their unique names in the alphabetical order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directory {
    name: Label,
    contents: BTreeMap<Label, DirectoryContents>,
}

/// `DirectoryContents` is an enumeration of what a [`Directory`] can contain
/// and is used for when we are iterating through a [`Directory`].
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
    pub fn label(&self) -> &Label {
        match self {
            DirectoryContents::File { name, .. } => name,
            DirectoryContents::Directory(directory) => directory.current(),
        }
    }
}

impl Directory {
    /// Create a root directory.
    ///
    /// This function is usually used for testing and demonstation purposes.
    pub fn root() -> Self {
        Directory {
            name: Label::root(),
            contents: BTreeMap::new(),
        }
    }

    /// Create a directory, similar to `root`, except with a given name.
    ///
    /// This function is usually used for testing and demonstation purposes.
    pub fn new(name: Label) -> Self {
        Directory {
            name,
            contents: BTreeMap::new(),
        }
    }

    /// Get the name of the current `Directory`.
    pub fn name(&self) -> &Label {
        &self.name
    }

    /// Add the `content` under `name` to the current `Directory`.
    /// If `name` already exists in this directory, then the previous contents
    /// are replaced.
    pub fn insert(&mut self, name: Label, content: DirectoryContents) {
        self.contents.insert(name, content);
    }

    /// Returns an iterator for the contents of the current directory.
    ///
    /// Note that the returned iterator only iterates the current level,
    /// without going recursively into sub-directories.
    pub fn contents(&self) -> impl Iterator<Item = &DirectoryContents> {
        self.contents.values()
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
    pub fn find_file(&self, path: Path) -> Option<&File> {
        let mut contents = &self.contents;
        let path_depth = path.0.len();
        for (idx, label) in path.iter().enumerate() {
            match contents.get(label) {
                Some(DirectoryContents::Directory(d)) => contents = &d.contents,
                Some(DirectoryContents::File { name: _, file }) => {
                    if idx + 1 == path_depth {
                        return Some(file);
                    } else {
                        break; // Abort: finding a file before the last label.
                    }
                },
                None => break, // Abort: a label not found.
            }
        }
        None
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
    pub fn find_directory(&self, path: Path) -> Option<&Self> {
        let mut found = None;
        let mut contents = &self.contents;

        for label in path.iter() {
            match contents.get(label) {
                Some(DirectoryContents::Directory(d)) => {
                    found = Some(d);
                    contents = &d.contents;
                },
                Some(DirectoryContents::File { .. }) => break, // Abort: should not be a file.
                None => break,                                 // Abort: a label not found.
            }
        }

        found
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
    pub fn current(&self) -> &Label {
        &self.name
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
        self.contents().fold(0, |size, item| {
            if let DirectoryContents::File { name: _, file } = item {
                size + file.size()
            } else {
                size
            }
        })
    }

    /// Insert a file into a directory, given the full path to file (file name
    /// inclusive) and the `File` itself.
    ///
    /// This function is usually used for testing and demonstation purposes.
    pub fn insert_file(&mut self, path: Path, file: File) {
        let name = path.0.last().clone();
        let f = DirectoryContents::File {
            name: name.clone(),
            file,
        };

        let mut contents = &mut self.contents;
        let path_depth = path.0.len() - 1; // exclude the last label: file name

        if path_depth == 0 {
            contents.insert(name, f);
        } else {
            for (idx, label) in path.iter().enumerate() {
                // if label does not exist, create a sub directory.
                if contents.get(label).is_none() {
                    let new_dir = Directory::new(label.clone());
                    contents.insert(label.clone(), DirectoryContents::Directory(new_dir));
                }

                match contents.get_mut(label) {
                    Some(DirectoryContents::Directory(d)) => {
                        contents = &mut d.contents;
                        if idx + 1 == path_depth {
                            // We are in the last directory level, insert the file.
                            contents.insert(name, f);
                            return;
                        }
                    },
                    Some(DirectoryContents::File { .. }) => return, // Abort: should not be a file.
                    None => return,
                }
            }
        }
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
}
