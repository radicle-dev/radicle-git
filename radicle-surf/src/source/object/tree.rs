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

//! Represents git object type 'tree', i.e. like directory entries in Unix.
//! See git [doc](https://git-scm.com/book/en/v2/Git-Internals-Git-Objects) for more details.

use std::path::Path;

#[cfg(feature = "serde")]
use serde::{
    ser::{SerializeStruct as _, Serializer},
    Serialize,
};

use crate::{
    file_system::{directory, Directory},
    git::{self, Repository},
    source::{commit, object::Error},
};

/// Result of a directory listing, carries other trees and blobs.
pub struct Tree {
    pub directory: Directory,
    pub commit: Option<commit::Header>,
    /// Entries listed in that tree result.
    pub entries: Vec<TreeEntry>,
}

impl Tree {
    /// Retrieve the [`Tree`] for the given `revision` and directory `prefix`.
    ///
    /// # Errors
    ///
    /// Will return [`Error`] if any of the surf interactions fail.
    pub fn new<P, R>(repo: &Repository, revision: &R, prefix: Option<&P>) -> Result<Tree, Error>
    where
        P: AsRef<Path>,
        R: git::Revision,
    {
        let prefix = prefix.map(|p| p.as_ref());

        let prefix_dir = match prefix {
            None => repo.root_dir(revision)?,
            Some(path) => repo
                .root_dir(revision)?
                .find_directory(&path, repo)?
                .ok_or_else(|| Error::PathNotFound(path.to_path_buf()))?,
        };

        let mut entries = prefix_dir
            .entries(repo)?
            .entries()
            .cloned()
            .map(TreeEntry::from)
            .collect::<Vec<_>>();
        entries.sort();

        let last_commit = if prefix.is_none() {
            let history = repo.history(revision)?;
            Some(commit::Header::from(history.head()))
        } else {
            None
        };

        Ok(Tree {
            entries,
            directory: prefix_dir,
            commit: last_commit,
        })
    }
}

#[cfg(feature = "serde")]
impl Serialize for Tree {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        const FIELDS: usize = 4;
        let mut state = serializer.serialize_struct("Tree", FIELDS)?;
        state.serialize_field("entries", &self.entries)?;
        state.serialize_field("lastCommit", &self.commit)?;
        state.serialize_field("name", &self.directory.name())?;
        state.serialize_field("path", &self.directory.location())?;
        state.end()
    }
}

/// Entry in a Tree result.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TreeEntry {
    pub entry: directory::Entry,
}

impl From<directory::Entry> for TreeEntry {
    fn from(entry: directory::Entry) -> Self {
        Self { entry }
    }
}

impl From<TreeEntry> for directory::Entry {
    fn from(TreeEntry { entry }: TreeEntry) -> Self {
        entry
    }
}

#[cfg(feature = "serde")]
impl Serialize for TreeEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        const FIELDS: usize = 4;
        let mut state = serializer.serialize_struct("TreeEntry", FIELDS)?;
        state.serialize_field("path", &self.entry.location())?;
        state.serialize_field("name", &self.entry.name())?;
        state.serialize_field("lastCommit", &None::<commit::Header>)?;
        state.serialize_field(
            "kind",
            match self.entry {
                directory::Entry::File(_) => "blob",
                directory::Entry::Directory(_) => "directory",
            },
        )?;
        state.end()
    }
}
