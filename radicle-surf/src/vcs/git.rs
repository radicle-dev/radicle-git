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

// Re-export git2 as sub-module
pub use git2::{self, Error as Git2Error, Time};
pub use radicle_git_ext::Oid;

/// Provides ways of selecting a particular reference/revision.
mod reference;
pub use reference::{ParseError, Ref, Rev};

mod repo;
pub use repo::{History, Repository, RepositoryRef};

pub mod error;

pub mod ext;

/// Provides the data for talking about branches.
pub mod branch;
pub use branch::{Branch, BranchName, BranchType};

/// Provides the data for talking about tags.
pub mod tag;
pub use tag::{Tag, TagName};

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

/// Determines whether to look for local or remote references or both.
pub enum RefScope {
    /// List all branches by default.
    All,
    /// List only local branches.
    Local,
    /// List only remote branches.
    Remote {
        /// Name of the remote. If `None`, then get the reference from all
        /// remotes.
        name: Option<String>,
    },
}

/// Turn an `Option<P>` into a [`RefScope`]. If the `P` is present then
/// this is set as the remote of the `RefScope`. Otherwise, it's local
/// branch.
impl<P> From<Option<P>> for RefScope
where
    P: ToString,
{
    fn from(peer_id: Option<P>) -> Self {
        peer_id.map_or(RefScope::Local, |peer_id| RefScope::Remote {
            // We qualify the remotes as the PeerId + heads, otherwise we would grab the tags too.
            name: Some(format!("{}/heads", peer_id.to_string())),
        })
    }
}
