// Copyright © 2019-2020 The Radicle Foundation <hello@radicle.foundation>
//
// This file is part of radicle-link, distributed under the GPLv3 with Radicle
// Linking Exception. For full terms see the included LICENSE file.

use thiserror::Error;

use super::*;

#[derive(Debug, Error)]
pub enum Init {
    #[error(transparent)]
    Config(#[from] config::Error),
    #[error(transparent)]
    Git(#[from] git2::Error),
    #[error("the current identifier '{current}' does not match the identifier used to create the storage '{actual}'")]
    OwnerMismatch { current: String, actual: String },
}

#[derive(Debug, Error)]
pub enum Transaction {
    #[error("error determining if {old} is an ancestor of {new} in within {name}")]
    Ancestry {
        name: RefString,
        new: Oid,
        old: Oid,
        #[source]
        source: git2::Error,
    },
    #[error("error committing update for storage")]
    Commit {
        #[source]
        source: git2::Error,
    },
    #[error("error locking reference '{reference}' when attempting to update storage")]
    Lock {
        reference: RefString,
        #[source]
        source: git2::Error,
    },
    #[error("error obtaining signature for '{name}' '{email}'")]
    Signature {
        name: String,
        email: String,
        #[source]
        source: git2::Error,
    },
    #[error("error setting the direct reference '{reference}' to the target '{target}'")]
    SetDirect {
        reference: RefString,
        target: Oid,
        #[source]
        source: git2::Error,
    },
    #[error("error setting the symbolic reference '{reference}' to the target '{target}'")]
    SetSymbolic {
        reference: RefString,
        target: RefString,
        #[source]
        source: git2::Error,
    },
    #[error("error removing the reference '{reference}'")]
    Remove {
        reference: RefString,
        #[source]
        source: git2::Error,
    },
}

#[derive(Debug, Error)]
pub enum Update {
    #[error("non-fast-forward update of {name} (current: {cur}, new: {new})")]
    NonFF { name: RefString, new: Oid, cur: Oid },
    #[error(transparent)]
    FindRef(#[from] read::error::FindRef),
    #[error(transparent)]
    Git(#[from] git2::Error),
    #[error("the symref target {0} is itself a symref")]
    TargetSymbolic(RefString),
    #[error(transparent)]
    Transaction(#[from] Transaction),
    #[error("rejected changing type of reference '{0}' from symbolic to direct")]
    TypeChange(RefString),
}
