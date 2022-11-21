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

//! ```
//! use nonempty::NonEmpty;
//! use radicle_surf::file_system::{Directory, File, Label, Path, SystemType};
//! use radicle_surf::file_system::unsound;
//! use radicle_surf::vcs::git::*;
//! use std::collections::HashMap;
//! use std::str::FromStr;
//! # use std::error::Error;
//!
//! # fn main() -> Result<(), Box<dyn Error>> {
//! let repo = Repository::new("./data/git-platinum")?;
//!
//! // Pin the browser to a parituclar commit.
//! let pin_commit = Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?;
//! let mut browser = Browser::new(&repo, Branch::local("master"))?;
//! browser.commit(pin_commit)?;
//!
//! let directory = browser.get_directory()?;
//! let mut directory_contents = directory.list_directory();
//! directory_contents.sort();
//!
//! assert_eq!(directory_contents, vec![
//!     SystemType::file(unsound::label::new(".i-am-well-hidden")),
//!     SystemType::file(unsound::label::new(".i-too-am-hidden")),
//!     SystemType::file(unsound::label::new("README.md")),
//!     SystemType::directory(unsound::label::new("bin")),
//!     SystemType::directory(unsound::label::new("src")),
//!     SystemType::directory(unsound::label::new("text")),
//!     SystemType::directory(unsound::label::new("this")),
//! ]);
//!
//! // find src directory in the Git directory and the in-memory directory
//! let src_directory = directory
//!     .find_directory(Path::new(unsound::label::new("src")))
//!     .expect("failed to find src");
//! let mut src_directory_contents = src_directory.list_directory();
//! src_directory_contents.sort();
//!
//! assert_eq!(src_directory_contents, vec![
//!     SystemType::file(unsound::label::new("Eval.hs")),
//!     SystemType::file(unsound::label::new("Folder.svelte")),
//!     SystemType::file(unsound::label::new("memory.rs")),
//! ]);
//! #
//! # Ok(())
//! # }
//! ```

use std::{convert::Infallible, str::FromStr};

// Re-export git2 as sub-module
pub use git2::{self, Error as Git2Error, Time};
use git_ref_format::{name::Components, Component, Qualified, RefString};
pub use radicle_git_ext::Oid;

mod repo;
pub use repo::{Error, Repository};

pub mod glob;
pub use glob::Glob;

mod history;
pub use history::History;

/// Provides the data for talking about branches.
pub mod branch;
pub use branch::{Branch, Local, Remote};

/// Provides the data for talking about tags.
pub mod tag;
pub use tag::Tag;

/// Provides the data for talking about commits.
pub mod commit;
pub use commit::{Author, Commit};

/// Provides the data for talking about namespaces.
pub mod namespace;
pub use namespace::Namespace;

/// Provides the data for talking about repository statistics.
pub mod stats;
pub use stats::Stats;

pub use crate::diff::Diff;

/// The signature of a commit
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Signature(Vec<u8>);

impl From<git2::Buf> for Signature {
    fn from(other: git2::Buf) -> Self {
        Signature((*other).into())
    }
}

/// Supports various ways to specify a revision used in Git.
pub trait Revision {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Returns the object id of this revision in `repo`.
    fn object_id(&self, repo: &Repository) -> Result<Oid, Self::Error>;
}

impl Revision for RefString {
    type Error = git2::Error;

    fn object_id(&self, repo: &Repository) -> Result<Oid, Self::Error> {
        repo.git2_repo().refname_to_id(self.as_str()).map(Oid::from)
    }
}

impl Revision for &RefString {
    type Error = git2::Error;

    fn object_id(&self, repo: &Repository) -> Result<Oid, Self::Error> {
        repo.git2_repo().refname_to_id(self.as_str()).map(Oid::from)
    }
}

impl Revision for Qualified<'_> {
    type Error = git2::Error;

    fn object_id(&self, repo: &Repository) -> Result<Oid, Self::Error> {
        repo.git2_repo().refname_to_id(self.as_str()).map(Oid::from)
    }
}

impl Revision for &Qualified<'_> {
    type Error = git2::Error;

    fn object_id(&self, repo: &Repository) -> Result<Oid, Self::Error> {
        repo.git2_repo().refname_to_id(self.as_str()).map(Oid::from)
    }
}

impl Revision for Oid {
    type Error = Infallible;

    fn object_id(&self, _repo: &Repository) -> Result<Oid, Self::Error> {
        Ok(*self)
    }
}

impl Revision for &str {
    type Error = git2::Error;

    fn object_id(&self, _repo: &Repository) -> Result<Oid, Self::Error> {
        Oid::from_str(self).map(Oid::from)
    }
}

impl Revision for &Branch {
    type Error = Error;

    fn object_id(&self, repo: &Repository) -> Result<Oid, Self::Error> {
        let refname = repo.namespaced_refname(&self.refname())?;
        Ok(repo.git2_repo().refname_to_id(&refname).map(Oid::from)?)
    }
}

impl Revision for &Tag {
    type Error = Infallible;

    fn object_id(&self, _repo: &Repository) -> Result<Oid, Self::Error> {
        Ok(self.id())
    }
}

/// A common trait for anything that can convert to a `Commit`.
pub trait ToCommit {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Converts to a commit in `repo`.
    fn to_commit(self, repo: &Repository) -> Result<Commit, Self::Error>;
}

impl ToCommit for Commit {
    type Error = Infallible;

    fn to_commit(self, _repo: &Repository) -> Result<Commit, Self::Error> {
        Ok(self)
    }
}

impl<R: Revision> ToCommit for R {
    type Error = Error;

    fn to_commit(self, repo: &Repository) -> Result<Commit, Self::Error> {
        let oid = repo.object_id(&self)?;
        let commit = repo.git2_repo().find_commit(oid.into())?;
        Ok(Commit::try_from(commit)?)
    }
}

pub(crate) fn refstr_join<'a>(c: Component<'a>, cs: Components<'a>) -> RefString {
    std::iter::once(c).chain(cs).collect::<RefString>()
}
