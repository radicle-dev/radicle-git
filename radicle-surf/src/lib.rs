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

#![deny(missing_docs, unused_import_braces, unused_qualifications, warnings)]

//! Welcome to `radicle-surf`!
//!
//! `radicle-surf` is a system to describe a file-system in a VCS world.
//! We have the concept of files and directories, but these objects can change
//! over time while people iterate on them. Thus, it is a file-system within
//! history and we, the user, are viewing the file-system at a particular
//! snapshot. Alongside this, we will wish to take two snapshots and view their
//! differences.
//!
//! Let's start surfing (and apologies for the `expect`s):
//!
//! ```
//! use radicle_surf::vcs::git;
//! use radicle_surf::file_system::{Label, Path, SystemType};
//! use radicle_surf::file_system::unsound;
//! use pretty_assertions::assert_eq;
//! use std::str::FromStr;
//! # use std::error::Error;
//!
//! # fn main() -> Result<(), Box<dyn Error>> {
//! // We're going to point to this repo.
//! let repo = git::Repository::new("./data/git-platinum")?;
//!
//! // Here we initialise a new Broswer for a the git repo.
//! let mut browser = git::Browser::new(&repo, git::Branch::local("master"))?;
//!
//! // Set the history to a particular commit
//! let commit = git::Oid::from_str("80ded66281a4de2889cc07293a8f10947c6d57fe")?;
//! browser.commit(commit)?;
//!
//! // Get the snapshot of the directory for our current HEAD of history.
//! let directory = browser.get_directory()?;
//!
//! // Let's get a Path to the memory.rs file
//! let memory = unsound::path::new("src/memory.rs");
//!
//! // And assert that we can find it!
//! assert!(directory.find_file(memory).is_some());
//!
//! let root_contents = directory.list_directory();
//!
//! assert_eq!(root_contents, vec![
//!     SystemType::file(unsound::label::new(".i-am-well-hidden")),
//!     SystemType::file(unsound::label::new(".i-too-am-hidden")),
//!     SystemType::file(unsound::label::new("README.md")),
//!     SystemType::directory(unsound::label::new("bin")),
//!     SystemType::directory(unsound::label::new("src")),
//!     SystemType::directory(unsound::label::new("text")),
//!     SystemType::directory(unsound::label::new("this")),
//! ]);
//!
//! let src = directory
//!     .find_directory(Path::new(unsound::label::new("src")))
//!     .expect("failed to find src");
//! let src_contents = src.list_directory();
//!
//! assert_eq!(src_contents, vec![
//!     SystemType::file(unsound::label::new("Eval.hs")),
//!     SystemType::file(unsound::label::new("Folder.svelte")),
//!     SystemType::file(unsound::label::new("memory.rs")),
//! ]);
//! #
//! # Ok(())
//! # }
//! ```
pub mod diff;
pub mod file_system;
pub mod vcs;

pub mod commit;
pub use commit::{commit, commits, Commit};

pub mod object;
pub use object::{blob, tree as objectTree, Blob, BlobContent, Info, ObjectType, Tree};

pub mod person;
pub use person::Person;

pub mod revision;
pub use revision::Revision;

#[cfg(feature = "syntax")]
pub mod syntax;
#[cfg(feature = "syntax")]
pub use syntax::SYNTAX_SET;

pub mod tree;

// Private modules
mod nonempty;

pub use crate::vcs::git;
