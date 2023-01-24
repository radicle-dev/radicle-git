// This file is part of radicle-git
// <https://github.com/radicle-dev/radicle-git>
//
// Copyright (C) 2019-2022 The Radicle Team <dev@radicle.xyz>
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

//! `radicle-surf` is a library to describe a Git repository as a file system.
//! It aims to provide an easy-to-use API to browse a repository via the concept
//! of files and directories for any given revision. It also allows the user to
//! diff any two different revisions.
//!
//! The main entry point of the API is [Repository].
//!
//! Let's start surfing!

pub extern crate git_ref_format;

extern crate radicle_git_ext as git_ext;

pub mod blob;
pub mod diff;
pub mod fs;
pub mod tree;

// Re-export git2 as sub-module
pub use git2::{self, Error as Git2Error, Time};
pub use radicle_git_ext::Oid;

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
pub use commit::{Author, Commit};

mod namespace;
pub use namespace::Namespace;

mod stats;
pub use stats::Stats;

mod revision;
pub use revision::{Revision, Signature, ToCommit};

mod refs;

mod error;
pub use error::Error;
