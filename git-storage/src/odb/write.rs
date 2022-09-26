// Copyright © 2022 The Radicle Link Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::error::Error;

use git_ext::Oid;
use git_ref_format::RefStr;

use super::{Commit, Object, Read, Tree};

/// Read-write access to a git odb.
///
/// See [`crate::Write`] for an implementation of this trait.
pub trait Write: Read {
    /// The error type for writing a blob to the odb.
    type WriteBlob: Error + Send + Sync + 'static;

    /// The error type for writing a commit to the odb.
    type WriteCommit: Error + Send + Sync + 'static;

    /// The error type for writing a tag to the odb.
    type WriteTag: Error + Send + Sync + 'static;

    /// The error type for writing a tree to the odb.
    type WriteTree: Error + Send + Sync + 'static;

    /// Write a [`super::Blob`] containing the `data` provided.
    fn write_blob(&self, data: &[u8]) -> Result<Oid, Self::WriteBlob>;

    /// Write a [`Commit`] that points to the given `tree` and has the provided
    /// `parents`.
    ///
    /// The signature of the [`Commit`] is expected to be provided by the
    /// implementor of the trait.
    ///
    /// The commit will not be associated with any reference. If this is
    /// required then you can use the [`Oid`] as the target for a
    /// [`crate::refdb::Update`].
    fn write_commit<'a>(
        &self,
        tree: &Tree,
        parents: &[&Commit<'a>],
        message: &str,
    ) -> Result<Oid, Self::WriteCommit>;

    /// Write a [`super::Tag`] that points to the given `target`.
    ///
    /// The signature of the [`super::Tag`] is expected to be provided by the
    /// implementor of the trait.
    ///
    /// No reference is created, however, the `name` is used for naming the
    /// [`super::Tag`] object.
    ///
    /// If a reference is required then you can use the [`Oid`] as the target
    /// for a [`crate::refdb::Update`].
    fn write_tag<R>(&self, name: R, target: &Object, message: &str) -> Result<Oid, Self::WriteTag>
    where
        R: AsRef<RefStr>;

    /// Write a [`super::Tree`] using the provided `builder`.
    // XXX: It's annoying that this exposes git2 but the effort of abstracting the
    // tree builder is too high
    // TODO: instead of passing the builder, we could use an enum of operations to
    // modify a builder
    fn write_tree(&self, builder: git2::TreeBuilder) -> Result<Oid, Self::WriteTree>;
}