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

use std::{
    collections::BTreeMap,
    convert::{Infallible, Into as _},
    path::{Path, PathBuf},
};

use git2::Blob;
use radicle_git_ext::{is_not_found_err, Oid};
use radicle_std_ext::result::ResultExt as _;

use crate::git::{Repository, Revision};

pub mod error {
    use thiserror::Error;

    #[derive(Debug, Error, PartialEq)]
    pub enum Directory {
        #[error(transparent)]
        Git(#[from] git2::Error),
        #[error(transparent)]
        Entry(#[from] Entry),
        #[error(transparent)]
        File(#[from] File),
    }

    #[derive(Debug, Error, PartialEq, Eq)]
    pub enum Entry {
        #[error("the entry name was not valid UTF-8")]
        Utf8Error,
    }

    #[derive(Debug, Error, PartialEq)]
    pub enum File {
        #[error(transparent)]
        Git(#[from] git2::Error),
    }
}

/// A `File` in a git repository.
///
/// The representation is lightweight and contains the [`Oid`] that
/// points to the git blob which is this file.
///
/// The name of a file can be retrieved via [`File::name`].
///
/// The [`FileContent`] of a file can be retrieved via
/// [`File::content`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct File {
    /// The name of the file.
    name: String,
    /// The relative path of the file, not including the `name`,
    /// in respect to the root of the git repository.
    prefix: PathBuf,
    /// The object identifier of the git blob of this file.
    id: Oid,
}

impl File {
    /// Construct a new `File`.
    ///
    /// The `path` must be the prefix location of the directory, and
    /// so should not end in `name`.
    ///
    /// The `id` must point to a git blob.
    pub(crate) fn new(name: String, prefix: PathBuf, id: Oid) -> Self {
        debug_assert!(
            !prefix.ends_with(&name),
            "prefix = {:?}, name = {}",
            prefix,
            name
        );
        Self { name, prefix, id }
    }

    /// The name of this `File`.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// The object identifier of this `File`.
    pub fn id(&self) -> Oid {
        self.id
    }

    /// Return the exact path for this `File`, including the `name` of
    /// the directory itself.
    ///
    /// The path is relative to the git repository root.
    pub fn path(&self) -> PathBuf {
        self.prefix.join(&self.name)
    }

    /// Return the [`Path`] where this `File` is located, relative to the
    /// git repository root.
    pub fn location(&self) -> &Path {
        &self.prefix
    }

    /// Get the [`FileContent`] for this `File`.
    ///
    /// # Errors
    ///
    /// This function will fail if it could not find the `git` blob
    /// for the `Oid` of this `File`.
    pub fn content<'a>(&self, repo: &'a Repository) -> Result<FileContent<'a>, error::File> {
        let blob = repo.git2_repo().find_blob(self.id.into())?;
        Ok(FileContent { blob })
    }
}

/// The contents of a [`File`].
///
/// To construct a `FileContent` use [`File::content`].
pub struct FileContent<'a> {
    blob: Blob<'a>,
}

impl<'a> FileContent<'a> {
    /// Return the file contents as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        self.blob.content()
    }

    /// Return the size of the file contents.
    pub fn size(&self) -> usize {
        self.blob.size()
    }

    /// Creates a `FileContent` using a blob.
    pub(crate) fn new(blob: Blob<'a>) -> Self {
        Self { blob }
    }
}

/// A representations of a [`Directory`]'s entries.
pub struct Entries {
    listing: BTreeMap<String, Entry>,
}

impl Entries {
    /// Return the name of each [`Entry`].
    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.listing.keys()
    }

    /// Return each [`Entry`].
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.listing.values()
    }

    /// Return each [`Entry`] and its name.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Entry)> {
        self.listing.iter()
    }
}

/// An `Entry` is either a [`File`] entry or a [`Directory`] entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// A file entry within a [`Directory`].
    File(File),
    /// A sub-directory of a [`Directory`].
    Directory(Directory),
}

impl Entry {
    /// Get a label for the `Entriess`, either the name of the [`File`]
    /// or the name of the [`Directory`].
    pub fn name(&self) -> &String {
        match self {
            Entry::File(file) => &file.name,
            Entry::Directory(directory) => directory.name(),
        }
    }

    pub(crate) fn from_entry(
        entry: &git2::TreeEntry,
        path: PathBuf,
    ) -> Result<Option<Self>, error::Entry> {
        let name = entry.name().ok_or(error::Entry::Utf8Error)?.to_string();
        let id = entry.id().into();
        Ok(entry.kind().and_then(|kind| match kind {
            git2::ObjectType::Tree => Some(Self::Directory(Directory::new(name, path, id))),
            git2::ObjectType::Blob => Some(Self::File(File::new(name, path, id))),
            _ => None,
        }))
    }
}

/// A `Directory` is the representation of a file system directory, for a given
/// [`git` tree][git-tree].
///
/// The name of a directory can be retrieved via [`File::name`].
///
/// The [`Entries`] of a directory can be retrieved via
/// [`Directory::entries`].
///
/// [git-tree]: https://git-scm.com/book/en/v2/Git-Internals-Git-Objects
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directory {
    /// The name of the directoy.
    name: String,
    /// The relative path of the directory, not including the `name`,
    /// in respect to the root of the git repository.
    prefix: PathBuf,
    /// The object identifier of the git tree of this directory.
    id: Oid,
}

impl Directory {
    /// Creates a directory given its `id`.
    ///
    /// The `name` and `prefix` are both set to be empty.
    ///
    /// The `id` must point to a `git` tree.
    pub(crate) fn root(id: Oid) -> Self {
        Self::new("".to_string(), PathBuf::new(), id)
    }

    /// Creates a directory given its `name` and `id`.
    ///
    /// The `path` must be the prefix location of the directory, and
    /// so should not end in `name`.
    ///
    /// The `id` must point to a `git` tree.
    pub(crate) fn new(name: String, prefix: PathBuf, id: Oid) -> Self {
        debug_assert!(
            name.is_empty() || !prefix.ends_with(&name),
            "prefix = {:?}, name = {}",
            prefix,
            name
        );
        Self { name, prefix, id }
    }

    /// Get the name of the current `Directory`.
    pub fn name(&self) -> &String {
        &self.name
    }

    /// The object identifier of this `File`.
    pub fn id(&self) -> Oid {
        self.id
    }

    /// Return the exact path for this `Directory`, including the `name` of the
    /// directory itself.
    ///
    /// The path is relative to the git repository root.
    pub fn path(&self) -> PathBuf {
        self.prefix.join(&self.name)
    }

    /// Return the [`Path`] where this `Directory` is located, relative to the
    /// git repository root.
    pub fn location(&self) -> &Path {
        &self.prefix
    }

    /// Return the [`Entries`] for this `Directory`'s `Oid`.
    ///
    /// The resulting `Entries` will only resolve to this
    /// `Directory`'s entries. Any sub-directories will need to be
    /// resolved independently.
    ///
    /// # Errors
    ///
    /// This function will fail if it could not find the `git` tree
    /// for the `Oid`.
    pub fn entries(&self, repo: &Repository) -> Result<Entries, error::Directory> {
        let tree = repo.git2_repo().find_tree(self.id.into())?;

        let mut entries = BTreeMap::new();
        let mut error = None;

        // Walks only the first level of entries.
        tree.walk(git2::TreeWalkMode::PreOrder, |path, entry| {
            match Entry::from_entry(entry, Path::new(path).to_path_buf()) {
                Ok(Some(entry)) => match entry {
                    Entry::File(_) => {
                        entries.insert(entry.name().clone(), entry);
                        git2::TreeWalkResult::Ok
                    },
                    Entry::Directory(_) => {
                        entries.insert(entry.name().clone(), entry);
                        // Skip nested directories
                        git2::TreeWalkResult::Skip
                    },
                },
                Ok(None) => git2::TreeWalkResult::Skip,
                Err(err) => {
                    error = Some(err);
                    git2::TreeWalkResult::Abort
                },
            }
        })?;

        match error {
            Some(err) => Err(err.into()),
            None => Ok(Entries { listing: entries }),
        }
    }

    /// Find the [`Entry`] found at `path`, if it exists.
    pub fn find_entry<P>(
        &self,
        path: &P,
        repo: &Repository,
    ) -> Result<Option<Entry>, crate::git::Error>
    where
        P: AsRef<Path>,
    {
        // Search the path in git2 tree.
        let git2_tree = repo.git2_repo().find_tree(self.id.into())?;
        let entry = git2_tree
            .get_path(path.as_ref())
            .map(Some)
            .or_matches::<git2::Error, _, _>(is_not_found_err, || Ok(None))?;

        Ok(entry
            .and_then(|entry| Entry::from_entry(&entry, path.as_ref().to_path_buf()).transpose())
            .transpose()
            .unwrap())
    }

    /// Find the `Oid`, for a [`File`], found at `path`, if it exists.
    pub fn find_file<P>(
        &self,
        path: &P,
        repo: &Repository,
    ) -> Result<Option<Oid>, crate::git::Error>
    where
        P: AsRef<Path>,
    {
        Ok(match self.find_entry(path, repo)? {
            Some(Entry::File(f)) => Some(f.id),
            _ => None,
        })
    }

    /// Find the `Directory` found at `path`, if it exists.
    pub fn find_directory<P>(
        &self,
        path: P,
        repo: &Repository,
    ) -> Result<Option<Self>, crate::git::Error>
    where
        P: AsRef<Path>,
    {
        Ok(match self.find_entry(&path, repo)? {
            Some(Entry::Directory(d)) => Some(d),
            _ => None,
        })
    }

    // TODO(fintan): This is going to be a bit trickier so going to leave it out for
    // now
    #[allow(dead_code)]
    fn fuzzy_find(_label: &Path) -> Vec<Self> {
        unimplemented!()
    }

    /// Get the total size, in bytes, of a `Directory`. The size is
    /// the sum of all files that can be reached from this `Directory`.
    pub fn size(&self, repo: &Repository) -> Result<usize, error::Directory> {
        self.traverse(repo, 0, &mut |size, entry| match entry {
            Entry::File(file) => Ok(size + file.content(repo)?.size()),
            Entry::Directory(dir) => Ok(size + dir.size(repo)?),
        })
    }

    /// Traverse the entire `Directory` using the `initial`
    /// accumulator and the function `f`.
    ///
    /// For each [`Entry::Directory`] this will recursively call
    /// [`Directory::traverse`] and obtain its [`Entries`].
    ///
    /// `Error` is the error type of the fallible function.
    /// `B` is the type of the accumulator.
    /// `F` is the fallible function that takes the accumulator and
    /// the next [`Entry`], possibly providing the next accumulator
    /// value.
    pub fn traverse<Error, B, F>(
        &self,
        repo: &Repository,
        initial: B,
        f: &mut F,
    ) -> Result<B, Error>
    where
        Error: From<error::Directory>,
        F: FnMut(B, &Entry) -> Result<B, Error>,
    {
        self.entries(repo)?
            .entries()
            .try_fold(initial, |acc, entry| match entry {
                Entry::File(_) => f(acc, entry),
                Entry::Directory(directory) => {
                    let acc = directory.traverse(repo, acc, f)?;
                    f(acc, entry)
                },
            })
    }
}

impl Revision for Directory {
    type Error = Infallible;

    fn object_id(&self, _repo: &Repository) -> Result<Oid, Self::Error> {
        Ok(self.id)
    }
}
