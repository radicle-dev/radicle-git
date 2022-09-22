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

use std::{convert::TryFrom as _, str::FromStr as _};

#[cfg(feature = "serialize")]
use serde::{
    ser::{SerializeStruct as _, Serializer},
    Serialize,
};

use crate::{
    commit,
    file_system,
    object::{Error, Info, ObjectType},
    revision::Revision,
    vcs::git::{Browser, Rev},
};

/// Result of a directory listing, carries other trees and blobs.
pub struct Tree {
    /// Absolute path to the tree object from the repo root.
    pub path: String,
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
    pub path: String,
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
    browser: &mut Browser<'_>,
    maybe_revision: Option<Revision<P>>,
    maybe_prefix: Option<String>,
) -> Result<Tree, Error>
where
    P: ToString,
{
    let maybe_revision = maybe_revision.map(Rev::try_from).transpose()?;
    let prefix = maybe_prefix.unwrap_or_default();

    if let Some(revision) = maybe_revision {
        browser.rev(revision)?;
    }

    let path = if prefix == "/" || prefix.is_empty() {
        file_system::Path::root()
    } else {
        file_system::Path::from_str(&prefix)?
    };

    let root_dir = browser.get_directory()?;
    let prefix_dir = if path.is_root() {
        root_dir
    } else {
        root_dir
            .find_directory(path.clone())
            .ok_or_else(|| Error::PathNotFound(path.clone()))?
    };
    let mut prefix_contents = prefix_dir.list_directory();
    prefix_contents.sort();

    let entries_results: Result<Vec<TreeEntry>, Error> = prefix_contents
        .iter()
        .map(|(label, system_type)| {
            let entry_path = if path.is_root() {
                file_system::Path::new(label.clone())
            } else {
                let mut p = path.clone();
                p.push(label.clone());
                p
            };
            let mut commit_path = file_system::Path::root();
            commit_path.append(entry_path.clone());

            let info = Info {
                name: label.to_string(),
                object_type: match system_type {
                    file_system::SystemType::Directory => ObjectType::Tree,
                    file_system::SystemType::File => ObjectType::Blob,
                },
                last_commit: None,
            };

            Ok(TreeEntry {
                info,
                path: entry_path.to_string(),
            })
        })
        .collect();

    let mut entries = entries_results?;

    // We want to ensure that in the response Tree entries come first. `Ord` being
    // derived on the enum ensures Variant declaration order.
    //
    // https://doc.rust-lang.org/std/cmp/trait.Ord.html#derivable
    entries.sort_by(|a, b| a.info.object_type.cmp(&b.info.object_type));

    let last_commit = if path.is_root() {
        Some(commit::Header::from(browser.get().first()))
    } else {
        None
    };
    let name = if path.is_root() {
        "".into()
    } else {
        let (_first, last) = path.split_last();
        last.to_string()
    };
    let info = Info {
        name,
        object_type: ObjectType::Tree,
        last_commit,
    };

    Ok(Tree {
        path: prefix,
        entries,
        info,
    })
}
