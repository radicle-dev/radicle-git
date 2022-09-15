// Copyright © 2022 The Radicle Link Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use super::*;

/// Read-only access to a git refdb.
///
/// See [`crate::Read`] for an implementation of this trait.
pub trait Read {
    /// The error type for finding an object in the refdb.
    type FindObj: Error + Send + Sync + 'static;

    /// The error type for finding a blob in the refdb.
    type FindBlob: Error + Send + Sync + 'static;

    /// The error type for finding a commit in the refdb.
    type FindCommit: Error + Send + Sync + 'static;

    /// The error type for finding a tag in the refdb.
    type FindTag: Error + Send + Sync + 'static;

    /// The error type for finding a tree in the refdb.
    type FindTree: Error + Send + Sync + 'static;

    /// Find the [`Object`] corresponding to the given `oid`.
    ///
    /// Returns `None` if the [`Object`] did not exist.
    fn find_object(&self, oid: Oid) -> Result<Option<Object>, Self::FindObj>;

    /// Find the [`Blob`] corresponding to the given `oid`.
    ///
    /// Returns `None` if the [`Blob`] did not exist.
    fn find_blob(&self, oid: Oid) -> Result<Option<Blob>, Self::FindBlob>;

    /// Find the [`Commit`] corresponding to the given `oid`.
    ///
    /// Returns `None` if the [`Commit`] did not exist.
    fn find_commit(&self, oid: Oid) -> Result<Option<Commit>, Self::FindCommit>;

    /// Find the [`Tag`] corresponding to the given `oid`.
    ///
    /// Returns `None` if the [`Tag`] did not exist.
    fn find_tag(&self, oid: Oid) -> Result<Option<Tag>, Self::FindTag>;

    /// Find the [`Object`] corresponding to the given `oid`.
    ///
    /// Returns `None` if the [`Tag`] did not exist.
    fn find_tree(&self, oid: Oid) -> Result<Option<Tree>, Self::FindTree>;
}
