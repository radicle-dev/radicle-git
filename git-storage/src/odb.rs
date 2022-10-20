// Copyright © 2022 The Radicle Link Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `odb` is separated into two traits: [`Read`] and [`Write`], providing
//! access to [git objects][objs].
//!
//! The [`Read`] trait provides functions for read-only access to the odb.
//! The [`Write`] trait provides functions for read and write access to the
//! odb, thus it implies the [`Read`] trait.
//!
//! The reason for separating these types of actions out is that one can infer
//! what kind of access a function has to the odb by looking at which trait it
//! is using.
//!
//! For implementations of these traits, this crate provides [`crate::Read`] and
//! [`crate::Write`] structs.
//!
//! [objs]: https://git-scm.com/book/en/v2/Git-Internals-Git-Objects

// TODO: this doesn't abstract over the git2 types very well, but it's too much
// hassle to massage that right now.

use std::{
    error::Error,
    path::{Path, PathBuf},
};

pub use git2::{Blob, Commit, Object, Tag, Tree};

use git_ext::Oid;

pub mod read;
pub use read::Read;

pub mod write;
pub use write::Write;

/// A `TreeBuilder` represents a series of operations to build a git tree, see
/// *Tree Objects* [here][git-objects].
///
/// The [`TreeEntry`] variants represent a limited set of the actions one can
/// perform [git-objects]: <https://git-scm.com/book/en/v2/Git-Internals-Git-Objects>
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TreeBuilder {
    entries: Vec<TreeEntry>,
}

impl TreeBuilder {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// [`TreeBuilder::insert`] creates a [`TreeEntry::Insert`] entry in the
    /// `TreeBuilder`.
    pub fn insert<P, M>(mut self, name: P, oid: Oid, mode: M) -> Self
    where
        P: AsRef<Path>,
        M: Into<i32>,
    {
        let name = name.as_ref().to_path_buf();
        let filemode = mode.into();
        self.entries.push(TreeEntry::Insert {
            name,
            oid,
            filemode,
        });
        self
    }

    /// [`TreeBuilder::remove`] creates a [`TreeEntry::Remove`] entry in the
    /// `TreeBuilder`.
    pub fn remove<P>(mut self, name: P) -> Self
    where
        P: AsRef<Path>,
    {
        let name = name.as_ref().to_path_buf();
        self.entries.push(TreeEntry::Remove { name });
        self
    }

    /// Iterate over the [`TreeEntry`]'s found in the `TreeBuilder`.
    pub fn iter(&self) -> impl Iterator<Item = &TreeEntry> {
        self.entries.iter()
    }
}

/// A `TreeEntry` represents a single operation within a [`TreeBuilder`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TreeEntry {
    /// Insert the `oid` under the `name` path in the resulting tree.
    /// The `filemode` determines the file mode for the file found at `name`.
    Insert {
        name: PathBuf,
        oid: Oid,
        filemode: i32,
    },
    /// Remove the `name` path from the resulting tree.
    Remove { name: PathBuf },
}

/// Find the [`Object`] corresponding to the given `oid`.
///
/// Will fail if the object does not exist.
pub fn object<S>(storage: &S, oid: Oid) -> Result<Object, S::FindObj>
where
    S: Read<FindObj = git2::Error>,
{
    storage
        .find_object(oid)?
        .ok_or_else(|| not_found("object", oid))
}

/// Find the [`Blob`] corresponding to the given `oid`.
///
/// Will fail if the blob does not exist.
pub fn blob<S>(storage: &S, oid: Oid) -> Result<Blob, S::FindBlob>
where
    S: Read<FindBlob = git2::Error>,
{
    storage
        .find_blob(oid)?
        .ok_or_else(|| not_found("blob", oid))
}

/// Find the [`Commit`] corresponding to the given `oid`.
///
/// Will fail if the commit does not exist.
pub fn commit<S>(storage: &S, oid: Oid) -> Result<Commit, S::FindCommit>
where
    S: Read<FindCommit = git2::Error>,
{
    storage
        .find_commit(oid)?
        .ok_or_else(|| not_found("commit", oid))
}

/// Find the [`Tag`] corresponding to the given `oid`.
///
/// Will fail if the tag does not exist.
pub fn tag<S>(storage: &S, oid: Oid) -> Result<Tag, S::FindTag>
where
    S: Read<FindTag = git2::Error>,
{
    storage.find_tag(oid)?.ok_or_else(|| not_found("tag", oid))
}

/// Find the [`Tree`] corresponding to the given `oid`.
///
/// Will fail if the tree does not exist.
pub fn tree<S>(storage: &S, oid: Oid) -> Result<Tree, S::FindTree>
where
    S: Read<FindTree = git2::Error>,
{
    storage
        .find_tree(oid)?
        .ok_or_else(|| not_found("tree", oid))
}

fn not_found(kind: &str, oid: Oid) -> git2::Error {
    use git2::{ErrorClass::*, ErrorCode::*};

    git2::Error::new(NotFound, Object, format!("could not find {kind} {oid}"))
}
