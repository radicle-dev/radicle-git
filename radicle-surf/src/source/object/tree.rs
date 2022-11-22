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

use std::path::{Path, PathBuf};

use git_ref_format::refname;
#[cfg(feature = "serialize")]
use serde::{
    ser::{SerializeStruct as _, Serializer},
    Serialize,
};

use crate::{
    file_system::directory,
    git::Repository,
    source::{
        commit,
        object::{Error, Info, ObjectType},
        revision::Revision,
    },
};

/// Result of a directory listing, carries other trees and blobs.
pub struct Tree {
    /// Absolute path to the tree object from the repo root.
    pub path: PathBuf,
    /// Entries listed in that tree result.
    pub entries: Vec<TreeEntry>,
    /// Extra info for the tree object.
    pub info: Info,
}

#[cfg(feature = "serialize")]
impl Serialize for Tree {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Tree", 3)?;
        state.serialize_field("path", &self.path)?;
        state.serialize_field("entries", &self.entries)?;
        state.serialize_field("info", &self.info)?;
        state.end()
    }
}

// TODO(xla): Ensure correct by construction.
/// Entry in a Tree result.
pub struct TreeEntry {
    /// Extra info for the entry.
    pub info: Info,
    /// Absolute path to the object from the root of the repo.
    pub path: PathBuf,
}

#[cfg(feature = "serialize")]
impl Serialize for TreeEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Tree", 2)?;
        state.serialize_field("path", &self.path)?;
        state.serialize_field("info", &self.info)?;
        state.end()
    }
}

/// Retrieve the [`Tree`] for the given `revision` and directory `prefix`.
///
/// # Errors
///
/// Will return [`Error`] if any of the surf interactions fail.
pub fn tree<P>(
    repo: &Repository,
    maybe_revision: Option<Revision>,
    maybe_prefix: Option<&P>,
) -> Result<Tree, Error>
where
    P: AsRef<Path>,
{
    let maybe_prefix = maybe_prefix.map(|p| p.as_ref());
    let rev = match maybe_revision {
        Some(r) => r,
        None => Revision::Branch {
            name: refname!("main"),
            remote: None,
        },
    };

    let prefix_dir = match maybe_prefix {
        None => repo.root_dir(&rev)?,
        Some(path) => repo
            .root_dir(&rev)?
            .find_directory(path, repo)?
            .ok_or_else(|| Error::PathNotFound(path.to_path_buf()))?,
    };

    let mut entries = prefix_dir
        .entries(repo)?
        .entries()
        .fold(Vec::new(), |mut entries, entry| {
            let entry_path = match maybe_prefix {
                Some(path) => {
                    let mut path = path.to_path_buf();
                    path.push(entry.name());
                    path
                },
                None => PathBuf::new(),
            };

            let info = Info {
                name: entry.name().clone(),
                object_type: match entry {
                    directory::Entry::Directory(_) => ObjectType::Tree,
                    directory::Entry::File { .. } => ObjectType::Blob,
                },
                last_commit: None,
            };

            entries.push(TreeEntry {
                info,
                path: entry_path,
            });
            entries
        });

    // We want to ensure that in the response Tree entries come first. `Ord` being
    // derived on the enum ensures Variant declaration order.
    //
    // https://doc.rust-lang.org/std/cmp/trait.Ord.html#derivable
    entries.sort_by(|a, b| a.info.object_type.cmp(&b.info.object_type));

    let last_commit = if maybe_prefix.is_none() {
        let history = repo.history(&rev)?;
        Some(commit::Header::from(history.head()))
    } else {
        None
    };
    let name = match maybe_prefix {
        None => "".into(),
        Some(path) => path.file_name().unwrap().to_str().unwrap().to_string(),
    };
    let info = Info {
        name,
        object_type: ObjectType::Tree,
        last_commit,
    };

    Ok(Tree {
        path: maybe_prefix.map_or(PathBuf::new(), |path| path.to_path_buf()),
        entries,
        info,
    })
}
