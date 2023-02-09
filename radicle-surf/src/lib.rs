// This file is part of radicle-git
// <https://github.com/radicle-dev/radicle-git>
//
// Copyright (C) 2019-2023 The Radicle Team <dev@radicle.xyz>
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

//! `radicle-surf` is a library to help users explore a Git repository with
//! ease. It supports browsing a repository via the concept of files and
//! directories, or via blobs and trees in a git fashion. With the additional
//! support of [`diff::Diff`] and [`History`], this library can be used to build
//! an intuitive UI for any Git repository.
//!
//! The main entry point of the library API is [`Repository`].
//!
//! Let's start surfing!
//!
//! ## Serialization with feature `serde`
//!
//! Many types in this crate support serialization using [`Serde`][serde]
//! through the `serde` feature flag for this crate.
//!
//! [serde]: https://crates.io/crates/serde

/// Re-exports.
pub use git_ref_format;

/// Represents an object id in Git. Re-exported from `radicle-git-ext`.
pub type Oid = radicle_git_ext::Oid;

pub mod blob;
pub mod diff;
pub mod fs;
pub mod tree;

/// Private modules with their public types.
mod repo;
pub use repo::Repository;

mod glob;
pub use glob::Glob;

mod history;
pub use history::History;

mod branch;
pub use branch::{Branch, Local, Remote};

mod tag;
pub use tag::Tag;

mod commit;
pub use commit::{Author, Commit, Time};

mod namespace;
pub use namespace::Namespace;

mod stats;
pub use stats::Stats;

mod revision;
pub use revision::{Revision, Signature, ToCommit};

mod refs;

mod error;
pub use error::Error;
