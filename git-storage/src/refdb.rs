// Copyright © 2022 The Radicle Link Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `refdb` is separated into two traits: [`Read`] and [`Write`], providing
//! access to [git references][refs].
//!
//! The [`Read`] trait provides functions for read-only access to the refdb.
//! The [`Write`] trait provides functions for read and write access to the
//! refdb, thus it implies the [`Read`] trait.
//!
//! The reason for separating these types of actions out is that one can infer
//! what kind of access a function has to the refdb by looking at which trait it
//! is using.
//!
//! For implementations of these traits, this crate provides [`crate::Read`] and
//! [`crate::Write`] structs.
//!
//! [refs]: https://git-scm.com/book/en/v2/Git-Internals-Git-References

use std::fmt::Debug;

use git_ext::Oid;
use git_ref_format::RefString;

pub mod iter;
pub use iter::{References, ReferencesGlob};

pub mod read;
pub use read::Read;

pub mod write;
pub use write::{previous, Applied, Policy, SymrefTarget, Update, Updated, Write};

/// A read-only structure representing a git reference.
///
/// References can be constructed by using the [`Read`] functions:
///   * [`Read::find_reference`]
///   * [`Read::find_references`]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Reference {
    /// The name of the reference.
    pub name: RefString,
    /// The target of the reference.
    pub target: Target,
}

impl<'a> TryFrom<git2::Reference<'a>> for Reference {
    type Error = error::ParseReference;

    fn try_from(value: git2::Reference) -> Result<Self, Self::Error> {
        use error::ParseReference;

        let name = value
            .name()
            .ok_or(ParseReference::InvalidUtf8)
            .and_then(|name| RefString::try_from(name).map_err(ParseReference::from))?;
        let target = match value.target() {
            None => {
                let name = value
                    .symbolic_target()
                    .ok_or(ParseReference::InvalidUtf8)
                    .and_then(|name| RefString::try_from(name).map_err(ParseReference::from))?;
                name.into()
            },
            Some(oid) => oid.into(),
        };

        Ok(Reference { name, target })
    }
}

/// The target a reference points to.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Target {
    /// The reference points directly at an `oid`.
    Direct { oid: Oid },
    /// The reference is symbolic and points to another reference.
    Symbolic { name: RefString },
}

impl From<Oid> for Target {
    fn from(oid: Oid) -> Self {
        Self::Direct { oid }
    }
}

impl From<git2::Oid> for Target {
    fn from(oid: git2::Oid) -> Self {
        Oid::from(oid).into()
    }
}

impl From<RefString> for Target {
    fn from(name: RefString) -> Self {
        Self::Symbolic { name }
    }
}

pub mod error {
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum ParseReference {
        #[error("reference name did contain valid UTF-8 bytes")]
        InvalidUtf8,
        #[error(transparent)]
        RefString(#[from] git_ref_format::Error),
    }

    #[derive(Debug, Error)]
    pub enum Iter {
        #[error(transparent)]
        Git(#[from] git2::Error),
        #[error(transparent)]
        Parse(#[from] ParseReference),
    }
}

/// Utility function for resolving a [`Reference`] to its [`Oid`]
pub(crate) fn resolve(repo: &git2::Repository, r: &Reference) -> Result<Oid, git2::Error> {
    match &r.target {
        Target::Direct { oid } => Ok(*oid),
        Target::Symbolic { name } => repo.refname_to_id(name.as_str()).map(Oid::from),
    }
}
