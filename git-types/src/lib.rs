// Copyright © 2019-2020 The Radicle Foundation <hello@radicle.foundation>
//
// This file is part of radicle-link, distributed under the GPLv3 with Radicle
// Linking Exception. For full terms see the included LICENSE file.

extern crate radicle_git_ext as git_ext;
extern crate radicle_std_ext as std_ext;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate radicle_macros;

pub mod namespace;
pub mod reference;
pub mod refspec;
pub mod remote;
pub mod urn;
pub use urn::Urn;

mod sealed;

pub use namespace::{AsNamespace, Namespace};
pub use reference::{
    AsRemote,
    Many,
    Multiple,
    One,
    Reference as GenericRef,
    RefsCategory,
    Single,
    SymbolicRef,
};
pub use refspec::{Fetchspec, Pushspec, Refspec};

/// Helper to aid type inference constructing a [`Reference`] without a
/// namespace.
pub struct Flat;

impl From<Flat> for Option<Namespace<git_ext::Oid>> {
    fn from(_flat: Flat) -> Self {
        None
    }
}

/// Type specialised reference for the most common use within this crate.
pub type Reference<C, R> = GenericRef<Namespace<git_ext::Oid>, R, C>;

/// Whether we should force the overwriting of a reference or not.
#[derive(Debug, Clone, Copy)]
pub enum Force {
    /// We should overwrite.
    True,
    /// We should not overwrite.
    False,
}

impl Force {
    /// Convert the Force to its `bool` equivalent.
    fn as_bool(&self) -> bool {
        match self {
            Force::True => true,
            Force::False => false,
        }
    }
}

impl From<bool> for Force {
    fn from(b: bool) -> Self {
        if b {
            Self::True
        } else {
            Self::False
        }
    }
}
