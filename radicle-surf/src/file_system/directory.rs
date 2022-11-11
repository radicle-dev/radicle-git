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

use crate::{
    file_system::{error::LabelError, path::*, Error},
    git::{self, RepositoryRef, Revision},
};
use git2::Blob;
use radicle_git_ext::Oid;
use std::{
    collections::BTreeMap,
    convert::{Infallible, TryFrom},
    path,
};

/// Represents a `file` in a git repo.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct File {
    pub(crate) name: Label,
    pub(crate) oid: Oid,
}

impl File {
    /// Create a new `File`.
    pub fn new(name: String, oid: Oid) -> Self {
        File {
            name: Label {
                label: name,
                hidden: false,
            },
            oid,
        }
    }

    /// Returns the file name reference.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}

/// Represents the actual content of a file.
pub struct FileContent<'a> {
    blob: Blob<'a>,
}

impl<'a> FileContent<'a> {
    /// Returns the file content as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        self.blob.content()
    }

    /// Returns the size of file
    pub fn size(&self) -> usize {
        self.blob.size()
    }

    /// Creates a `FileContent` using a blob.
    pub(crate) fn new(blob: Blob<'a>) -> Self {
        Self { blob }
    }
}

/// Represents the listing of a directory.
pub struct DirectoryContent {
    listing: BTreeMap<Label, DirectoryEntry>,
}

impl DirectoryContent {
    /// Returns an iterator for the listing of a directory.
    pub fn iter(&self) -> impl Iterator<Item = &DirectoryEntry> {
        self.listing.values()
    }
}

/// Represents an entry in a directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectoryEntry {
    /// When the entry is a file.
    File(File),
    /// When the entry is a directory.
    Directory(Directory),
}

impl DirectoryEntry {
    /// Returns the label (short name, without the parent path) of the entry.
    pub fn label(&self) -> &Label {
        match self {
            DirectoryEntry::File(file) => &file.name,
            DirectoryEntry::Directory(directory) => directory.name(),
        }
    }
}

/// A `Directory` is a set of entries of sub-directories and files, ordered
/// by their unique names in the alphabetical order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directory {
    pub(crate) name: Label,
    pub(crate) oid: Oid,
}

impl Directory {
    /// Creates a directory given `name` and `oid`, with empty contents.
    pub fn new(name: Label, oid: Oid) -> Self {
        Directory { name, oid }
    }

    /// Get the name of the current `Directory`.
    pub fn name(&self) -> &Label {
        &self.name
    }

    /// Returns a `DirectoryContent` for the current directory.
    pub fn contents(&self, repo: &RepositoryRef) -> Result<DirectoryContent, git::Error> {
        let listing = repo.directory_get(self)?;
        Ok(DirectoryContent { listing })
    }

    /// Retrieves a `DirectoryEntry` for `path` in `repo`.
    pub fn get_path(
        &self,
        path: &path::Path,
        repo: &RepositoryRef,
    ) -> Result<DirectoryEntry, git::Error> {
        // Search the path in git2 tree.
        let git2_tree = repo.repo_ref.find_tree(self.oid.into())?;
        let entry = git2_tree.get_path(path)?;

        // Construct the DirectoryEntry.
        let name = entry.name().ok_or_else(|| {
            Error::Label(LabelError::InvalidUTF8 {
                label: String::from_utf8_lossy(entry.name_bytes()).into(),
            })
        })?;
        let label = Label {
            label: name.to_string(),
            hidden: false,
        };
        let oid: Oid = entry.id().into();
        match entry.kind() {
            Some(git2::ObjectType::Tree) => {
                let dir = Directory::new(label, oid);
                Ok(DirectoryEntry::Directory(dir))
            },
            Some(git2::ObjectType::Blob) => {
                let f = File { name: label, oid };
                Ok(DirectoryEntry::File(f))
            },
            _ => {
                let file_path = Path::try_from(path.to_path_buf())?;
                Err(git::Error::PathNotFound(file_path))
            },
        }
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
    pub fn find_file(&self, path: Path, repo: &RepositoryRef) -> Option<Oid> {
        let path_buf: std::path::PathBuf = (&path).into();
        let entry = match self.get_path(path_buf.as_path(), repo) {
            Ok(entry) => entry,
            Err(_) => return None,
        };

        match entry {
            DirectoryEntry::File(f) => Some(f.oid),
            DirectoryEntry::Directory(_) => None,
        }
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
    pub fn find_directory(&self, path: Path, repo: &RepositoryRef) -> Option<Self> {
        let path_buf: std::path::PathBuf = (&path).into();
        let entry = match self.get_path(path_buf.as_path(), repo) {
            Ok(entry) => entry,
            Err(_) => return None,
        };

        match entry {
            DirectoryEntry::File(_) => None,
            DirectoryEntry::Directory(d) => Some(d),
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
    pub fn size(&self, repo: &RepositoryRef) -> Result<usize, git::Error> {
        let mut size = 0;
        let contents = self.contents(repo)?;
        for item in contents.iter() {
            match item {
                DirectoryEntry::File(f) => {
                    let sz = repo.file_size(f.oid)?;
                    size += sz;
                },
                DirectoryEntry::Directory(d) => {
                    size += d.size(repo)?;
                },
            }
        }
        Ok(size)
    }
}

impl Revision for Directory {
    type Error = Infallible;

    fn object_id(&self, _repo: &RepositoryRef) -> Result<Oid, Self::Error> {
        Ok(self.oid)
    }
}
