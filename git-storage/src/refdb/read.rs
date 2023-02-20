// Copyright © 2022 The Radicle Link Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::error::Error;

use git_ext::{
    ref_format::{refspec, RefStr},
    Oid,
};

use super::Reference;

/// Read-only access to a git refdb.
///
/// See [`crate::Read`] for an implementation of this trait.
pub trait Read {
    /// The error type for finding a reference in the refdb.
    type FindRef: Error + Send + Sync + 'static;

    /// The error type for finding references in the refdb.
    type FindRefs: Error + Send + Sync + 'static;

    /// The error type for finding reference Oid in the refdb.
    type FindRefOid: Error + Send + Sync + 'static;

    /// Iterator for references returned by `find_references`.
    type References: Iterator<Item = Result<Reference, Self::FindRefs>>;

    /// Find the reference that corresponds to `name`. If the reference does not
    /// exist, then `None` is returned.
    fn find_reference<Ref>(&self, name: Ref) -> Result<Option<Reference>, Self::FindRef>
    where
        Ref: AsRef<RefStr>;

    /// Find the references that match `pattern`.
    fn find_references<Pat>(&self, pattern: Pat) -> Result<Self::References, Self::FindRefs>
    where
        Pat: AsRef<refspec::PatternStr>;

    /// Find the [`Oid`] for the reference that corresponds to `name`. If the
    /// reference does not exist, then `None` is returned.
    fn find_reference_oid<Ref>(&self, name: Ref) -> Result<Option<Oid>, Self::FindRefOid>
    where
        Ref: AsRef<RefStr>;
}
