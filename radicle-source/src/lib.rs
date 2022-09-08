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

//! Source code related functionality.

/// To avoid incompatible versions of `radicle-surf`, `radicle-source`
/// re-exports the package under the `surf` alias.
pub use radicle_surf as surf;

pub mod branch;
pub use branch::{branches, local_state, Branch, LocalState};

pub mod commit;
pub use commit::{commit, commits, Commit};

pub mod error;
pub use error::Error;

pub mod object;
pub use object::{blob, tree, Blob, BlobContent, Info, ObjectType, Tree};

pub mod oid;
pub use oid::Oid;

pub mod person;
pub use person::Person;

pub mod revision;
pub use revision::Revision;

#[cfg(feature = "syntax")]
pub mod syntax;
#[cfg(feature = "syntax")]
pub use syntax::SYNTAX_SET;

pub mod tag;
pub use tag::{tags, Tag};
