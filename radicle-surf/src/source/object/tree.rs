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

use std::cmp::Ordering;

use radicle_git_ext::Oid;
#[cfg(feature = "serde")]
use serde::{
    ser::{SerializeStruct as _, Serializer},
    Serialize,
};

use crate::{file_system::directory, source::commit};

/// Represents a tree object as in git. It is essentially the content of
/// one directory. Note that multiple directories can have the same content,
/// i.e. have the same tree object. Hence this struct does not embed its path.
#[derive(Clone, Debug)]
pub struct Tree {
    /// The object id of this tree.
    id: Oid,
    entries: Vec<TreeEntry>,
    /// The commit object that created this tree object.
    commit: commit::Header,
}

impl Tree {
    /// Creates a new tree.
    pub(crate) fn new(id: Oid, entries: Vec<TreeEntry>, commit: commit::Header) -> Self {
        Self {
            id,
            entries,
            commit,
        }
    }

    pub fn object_id(&self) -> Oid {
        self.id
    }

    /// Returns the commit that created this tree.
    pub fn commit(&self) -> &commit::Header {
        &self.commit
    }

    /// Returns the entries of the tree.
    pub fn entries(&self) -> &Vec<TreeEntry> {
        &self.entries
    }
}

#[cfg(feature = "serde")]
impl Serialize for Tree {
    /// Sample output:
    /// (for `<entry_1>` and `<entry_2>` sample output, see [`TreeEntry`])
    /// ```
    /// {
    ///   "entries": [
    ///     { <entry_1> },
    ///     { <entry_2> },
    ///   ],
    ///   "lastCommit": {
    ///     "author": {
    ///       "email": "foobar@gmail.com",
    ///       "name": "Foo Bar"
    ///     },
    ///     "committer": {
    ///       "email": "noreply@github.com",
    ///       "name": "GitHub"
    ///     },
    ///     "committerTime": 1582198877,
    ///     "description": "A sample commit.",
    ///     "sha1": "b57846bbc8ced6587bf8329fc4bce970eb7b757e",
    ///     "summary": "Add a new sample"
    ///   },
    ///   "oid": "dd52e9f8dfe1d8b374b2a118c25235349a743dd2"
    /// }
    /// ```
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        const FIELDS: usize = 4;
        let mut state = serializer.serialize_struct("Tree", FIELDS)?;
        state.serialize_field("oid", &self.id)?;
        state.serialize_field("entries", &self.entries)?;
        state.serialize_field("lastCommit", &self.commit)?;
        state.end()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Entry {
    Tree(Oid),
    Blob(Oid),
}

/// Entry in a Tree result.
#[derive(Clone, Debug)]
pub struct TreeEntry {
    name: String,
    entry: Entry,

    /// The commit object that created this entry object.
    commit: commit::Header,
}

impl TreeEntry {
    pub(crate) fn new(name: String, entry: Entry, commit: commit::Header) -> Self {
        Self {
            name,
            entry,
            commit,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn entry(&self) -> &Entry {
        &self.entry
    }

    pub fn is_tree(&self) -> bool {
        matches!(self.entry, Entry::Tree(_))
    }

    pub fn commit(&self) -> &commit::Header {
        &self.commit
    }

    pub fn object_id(&self) -> Oid {
        match self.entry {
            Entry::Blob(id) => id,
            Entry::Tree(id) => id,
        }
    }
}

// To support `sort`.
impl Ord for TreeEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for TreeEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for TreeEntry {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for TreeEntry {}

impl From<directory::Entry> for Entry {
    fn from(entry: directory::Entry) -> Self {
        match entry {
            directory::Entry::File(f) => Entry::Blob(f.id()),
            directory::Entry::Directory(d) => Entry::Tree(d.id()),
        }
    }
}

#[cfg(feature = "serde")]
impl Serialize for TreeEntry {
    /// Sample output:
    /// ```
    ///  {
    ///     "kind": "blob",
    ///     "lastCommit": {
    ///       "author": {
    ///         "email": "foobar@gmail.com",
    ///         "name": "Foo Bar"
    ///       },
    ///       "committer": {
    ///         "email": "noreply@github.com",
    ///         "name": "GitHub"
    ///       },
    ///       "committerTime": 1578309972,
    ///       "description": "This is a sample file",
    ///       "sha1": "2873745c8f6ffb45c990eb23b491d4b4b6182f95",
    ///       "summary": "Add a new sample"
    ///     },
    ///     "name": "Sample.rs",
    ///     "oid": "6d6240123a8d8ea8a8376610168a0a4bcb96afd0"
    ///   },
    /// ```
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        const FIELDS: usize = 4;
        let mut state = serializer.serialize_struct("TreeEntry", FIELDS)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field(
            "kind",
            match self.entry {
                Entry::Blob(_) => "blob",
                Entry::Tree(_) => "tree",
            },
        )?;
        state.serialize_field("oid", &self.object_id())?;
        state.serialize_field("lastCommit", &self.commit)?;
        state.end()
    }
}
