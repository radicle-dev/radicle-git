// Copyright © 2022 The Radicle Link Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::error::Error;

use git_ext::{
    ref_format::{Qualified, RefString},
    Oid,
};

use super::read::Read;

pub mod previous;

/// Read-write access to a git refdb.
///
/// See [`crate::Write`] for an implementation of this trait.
pub trait Write: Read {
    type UpdateError: Error + Send + Sync + 'static;

    /// Apply the provided `updates` to the refdb.
    ///
    /// The implementation of `update` is expected to be transactional. All
    /// successful updates are returned in [`Applied::updated`] while rejected
    /// updates should be returned in [`Applied::rejected`].
    fn update<'a, U>(&mut self, updates: U) -> Result<Applied<'a>, Self::UpdateError>
    where
        U: IntoIterator<Item = Update<'a>>;
}

/// Perform an update to a reference in the refdb.
#[derive(Debug, Clone)]
pub enum Update<'a> {
    /// Update a direct reference, i.e. one that points directly to an [`Oid`].
    Direct {
        /// The `name` of the reference.
        name: Qualified<'a>,
        /// The `target` to point the reference at.
        target: Oid,
        /// Policy to apply when an [`Update`] would not apply as a
        /// fast-forward.
        ///
        /// An update is a fast-forward iff:
        ///
        /// 1. A ref with the same name already exists
        /// 2. The ref is a direct ref, and the update is a [`Update::Direct`]
        /// 3. Both the existing and the update [`Oid`] point to a commit object
        ///    without peeling
        /// 4. The existing commit is an ancestor of the update commit
        ///
        /// or:
        ///
        /// 1. A ref with the same name does not already exist
        no_ff: Policy,
        /// The expectation of the previous value for the reference before
        /// making the update. This allows the update to be rejected in
        /// the case of a concurrent modification the reference.
        previous: previous::Edit,
        /// The [`reflog`][reflog] entry for the update.
        ///
        /// [reflog]: https://git-scm.com/docs/git-reflog
        // TODO: we may want to force the creation of a reflog entry for specific references.
        reflog: String,
    },
    Symbolic {
        /// The `name` of the reference.
        name: Qualified<'a>,
        /// The `target` to point the reference at. Currently, only supports
        /// one-level deep.
        target: SymrefTarget<'a>,
        /// Policy to apply when the ref already exists, but is a direct ref
        /// before the update.
        type_change: Policy,
        /// The expectation of the previous value for the reference before
        /// making the update. This allows the update to be rejected in
        /// the case of a concurrent modification the reference.
        previous: previous::Edit,
        /// The [`reflog`][reflog] entry for the update.
        ///
        /// [reflog]: https://git-scm.com/docs/git-reflog
        reflog: String,
    },
    Remove {
        /// The `name` of the reference.
        name: Qualified<'a>,
        /// The expectation of the previous value for the reference before
        /// making the update. This allows the update to be rejected in
        /// the case of a concurrent modification the reference.
        previous: previous::Remove,
    },
}

/// A target of a [symbolic reference][symref].
///
/// [symref]: https://git-scm.com/docs/git-symbolic-ref
#[derive(Debug, Clone)]
pub struct SymrefTarget<'a> {
    /// The `name` of the symbolic reference.
    pub name: Qualified<'a>,
    /// The underlying `target` of the symbolic reference.
    pub target: Oid,
}

/// The successful result of an [`Update`] applied to the refdb.
#[derive(Debug, Clone)]
pub enum Updated {
    /// The [`Update::Direct`] was succesful.
    Direct {
        /// The `name` of the reference that was updated.
        name: RefString,
        /// The new `target` of the reference that was updated.
        target: Oid,
        /// The previous target of the reference, if it existed.
        previous: Option<Oid>,
    },
    /// The [`Update::Symbolic`] was succesful.
    Symbolic {
        /// The `name` of the reference that was updated.
        name: RefString,
        /// The new `target` of the reference that was updated.
        target: RefString,
        /// The previous peeled target of the reference, if it existed.
        previous: Option<Oid>,
    },
    /// The [`Update::Remove`] was succesful.
    Removed {
        /// The `name` of the reference that was removed.
        name: RefString,
        /// The old `target` of the reference that was removed.
        previous: Oid,
    },
}

/// The outcome of running a [`Write::update`].
#[derive(Clone, Default)]
pub struct Applied<'a> {
    /// Any [`Update`]s that were rejected due to their [`previous::Edit`],
    /// [`previous::Remove`], or [`Policy`].
    pub rejected: Vec<Update<'a>>,
    /// The successful [`Update`]s applied to the refdb.
    pub updated: Vec<Updated>,
}

/// The policy to use when guarding against fast-forwards in the case of
/// [`Update::Direct`] and type changes in the case of [`Update::Symbolic`].
#[derive(Clone, Copy, Debug)]
pub enum Policy {
    /// Abort the entire transaction.
    Abort,
    /// Reject this update, but continue the transaction.
    Reject,
    /// Allow the update.
    Allow,
}
