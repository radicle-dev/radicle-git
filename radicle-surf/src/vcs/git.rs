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
pub use git2::{self, Error as Git2Error, Oid, Time};

/// Provides ways of selecting a particular reference/revision.
mod reference;
pub use reference::{Ref, Rev};

mod repo;
pub use repo::{History, Repository, RepositoryRef};

pub mod error;

mod ext;

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

use crate::{
    file_system,
    file_system::directory,
    vcs,
    vcs::{git::error::*, Vcs},
};
use nonempty::NonEmpty;
use std::{
    collections::{BTreeSet, HashMap},
    convert::TryFrom,
    str,
};

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

/// A [`crate::vcs::Browser`] that uses [`Repository`] as the underlying
/// repository backend, [`git2::Commit`] as the artifact, and [`Error`] for
/// error reporting.
pub type Browser<'a> = vcs::Browser<RepositoryRef<'a>, Commit, Error>;

impl<'a> Browser<'a> {
    /// Create a new browser to interact with.
    ///
    /// The `revspec` provided will be used to kick off the [`History`] for this
    /// `Browser`.
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Browser, Branch, Repository};
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let browser = Browser::new(&repo, Branch::local("master"))?;
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        repository: impl Into<RepositoryRef<'a>>,
        rev: impl Into<Rev>,
    ) -> Result<Self, Error> {
        let repository = repository.into();
        let history = repository.get_history(rev.into())?;
        Ok(Self::init(repository, history))
    }

    /// Create a new `Browser` that starts in a given `namespace`.
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Browser, Repository, Branch, RefScope, BranchName, Namespace};
    /// use std::convert::TryFrom;
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let browser = Browser::new_with_namespace(
    ///     &repo,
    ///     &Namespace::try_from("golden")?,
    ///     Branch::local("master")
    /// )?;
    ///
    /// let mut branches = browser.list_branches(RefScope::Local)?;
    /// branches.sort();
    ///
    /// assert_eq!(
    ///     branches,
    ///     vec![
    ///         Branch::local("banana"),
    ///         Branch::local("master"),
    ///     ]
    /// );
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_with_namespace(
        repository: impl Into<RepositoryRef<'a>>,
        namespace: &Namespace,
        rev: impl Into<Rev>,
    ) -> Result<Self, Error> {
        let repository = repository.into();
        // This is a bit weird, the references don't seem to all be present unless we
        // make a call to `references` o_O.
        let _ = repository.repo_ref.references()?;
        repository.switch_namespace(&namespace.to_string())?;
        let history = repository.get_history(rev.into())?;
        Ok(Self::init(repository, history))
    }

    fn init(repository: RepositoryRef<'a>, history: History) -> Self {
        let snapshot = Box::new(|repository: &RepositoryRef<'a>, history: &History| {
            let tree = Self::get_tree(repository.repo_ref, history.0.first())?;
            Ok(directory::Directory::from_hash_map(tree))
        });
        vcs::Browser {
            snapshot,
            history,
            repository,
        }
    }

    /// Switch the namespace you are browsing in. This will consume the previous
    /// `Browser` and give you back a new `Browser` for that particular
    /// namespace. The `revision` provided will kick-off the history for
    /// this `Browser`.
    pub fn switch_namespace(
        self,
        namespace: &Namespace,
        rev: impl Into<Ref>,
    ) -> Result<Self, Error> {
        self.repository.switch_namespace(&namespace.to_string())?;
        let history = self.get_history(Rev::from(rev))?;
        Ok(Browser {
            snapshot: self.snapshot,
            repository: self.repository,
            history,
        })
    }

    /// What is the current namespace we're browsing in.
    pub fn which_namespace(&self) -> Result<Option<Namespace>, Error> {
        self.repository
            .repo_ref
            .namespace_bytes()
            .map(Namespace::try_from)
            .transpose()
    }

    /// Set the current `Browser` history to the `HEAD` commit of the underlying
    /// repository.
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Browser, Repository, Branch};
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(&repo, Branch::local("master"))?;
    ///
    /// // ensure we're at HEAD
    /// browser.head();
    ///
    /// let directory = browser.get_directory();
    ///
    /// // We are able to render the directory
    /// assert!(directory.is_ok());
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn head(&mut self) -> Result<(), Error> {
        let history = self.repository.head()?;
        self.set(history);
        Ok(())
    }

    /// Set the current `Browser`'s [`History`] to the given [`BranchName`]
    /// provided.
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    /// * [`error::Error::NotBranch`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Branch, Browser, Repository};
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(&repo, Branch::local("master"))?;
    ///
    /// // ensure we're on 'master'
    /// browser.branch(Branch::local("master"));
    ///
    /// let directory = browser.get_directory();
    ///
    /// // We are able to render the directory
    /// assert!(directory.is_ok());
    /// #
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Branch, Browser, Repository};
    /// use radicle_surf::file_system::{Label, Path, SystemType};
    /// use radicle_surf::file_system::unsound;
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(&repo, Branch::local("master"))?;
    /// browser.branch(Branch::remote("dev", "origin"))?;
    ///
    /// let directory = browser.get_directory()?;
    /// let mut directory_contents = directory.list_directory();
    /// directory_contents.sort();
    ///
    /// assert!(directory_contents.contains(
    ///     &SystemType::file(unsound::label::new("here-we-are-on-a-dev-branch.lol"))
    /// ));
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn branch(&mut self, branch: Branch) -> Result<(), Error> {
        let name = BranchName(branch.name());
        self.set(self.repository.reference(branch, |reference| {
            let is_branch = ext::is_branch(reference) || reference.is_remote();
            if !is_branch {
                Some(Error::NotBranch(name))
            } else {
                None
            }
        })?);
        Ok(())
    }

    /// Set the current `Browser`'s [`History`] to the [`TagName`] provided.
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    /// * [`error::Error::NotTag`]
    ///
    /// # Examples
    ///
    /// ```
    /// use nonempty::NonEmpty;
    /// use radicle_surf::vcs::History;
    /// use radicle_surf::vcs::git::{TagName, Branch, Browser, Oid, Repository};
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(&repo, Branch::local("master"))?;
    ///
    /// // Switch to "v0.3.0"
    /// browser.tag(TagName::new("v0.3.0"))?;
    ///
    /// let expected_history = History(NonEmpty::from((
    ///     Oid::from_str("19bec071db6474af89c866a1bd0e4b1ff76e2b97")?,
    ///     vec![
    ///         Oid::from_str("f3a089488f4cfd1a240a9c01b3fcc4c34a4e97b2")?,
    ///         Oid::from_str("2429f097664f9af0c5b7b389ab998b2199ffa977")?,
    ///         Oid::from_str("d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3")?,
    ///     ]
    /// )));
    ///
    /// let history_ids = browser.get().map(|commit| commit.id);
    ///
    /// // We are able to render the directory
    /// assert_eq!(history_ids, expected_history);
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn tag(&mut self, tag_name: TagName) -> Result<(), Error> {
        let name = tag_name.clone();
        self.set(self.repository.reference(tag_name, |reference| {
            if !ext::is_tag(reference) {
                Some(Error::NotTag(name))
            } else {
                None
            }
        })?);
        Ok(())
    }

    /// Set the current `Browser`'s [`History`] to the [`Oid`] (SHA digest)
    /// provided.
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::{Label, SystemType};
    /// use radicle_surf::file_system::unsound;
    /// use radicle_surf::vcs::git::{Branch, Browser, Oid, Repository};
    /// use std::str::FromStr;
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(&repo, Branch::local("master"))?;
    ///
    /// // Set to the initial commit
    /// let commit = Oid::from_str("e24124b7538658220b5aaf3b6ef53758f0a106dc")?;
    ///
    /// browser.commit(commit)?;
    ///
    /// let directory = browser.get_directory()?;
    /// let mut directory_contents = directory.list_directory();
    ///
    /// assert_eq!(
    ///     directory_contents,
    ///     vec![
    ///         SystemType::file(unsound::label::new("README.md")),
    ///         SystemType::directory(unsound::label::new("bin")),
    ///         SystemType::directory(unsound::label::new("src")),
    ///         SystemType::directory(unsound::label::new("this")),
    ///     ]
    /// );
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn commit(&mut self, oid: Oid) -> Result<(), Error> {
        self.set(self.get_history(Rev::Oid(oid))?);
        Ok(())
    }

    /// Set a `Browser`'s [`History`] based on a [revspec](https://git-scm.com/docs/git-rev-parse.html#_specifying_revisions).
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    /// * [`error::Error::RevParseFailure`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::{Label, SystemType};
    /// use radicle_surf::file_system::unsound;
    /// use radicle_surf::vcs::git::{Browser, Branch, Oid, Repository};
    /// use std::str::FromStr;
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(&repo, Branch::local("master"))?;
    ///
    /// browser.rev(Branch::remote("dev", "origin"))?;
    ///
    /// let directory = browser.get_directory()?;
    /// let mut directory_contents = directory.list_directory();
    /// directory_contents.sort();
    ///
    /// assert!(directory_contents.contains(
    ///     &SystemType::file(unsound::label::new("here-we-are-on-a-dev-branch.lol"))
    /// ));
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn rev(&mut self, rev: impl Into<Rev>) -> Result<(), Error> {
        let history = self.get_history(rev.into())?;
        self.set(history);
        Ok(())
    }

    /// Parse an [`Oid`] from the given string. This is useful if we have a
    /// shorthand version of the `Oid`, as opposed to the full one.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Branch, Browser, Oid, Repository};
    /// use std::str::FromStr;
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(&repo, Branch::local("master"))?;
    ///
    /// // Set to the initial commit
    /// let commit = Oid::from_str("e24124b7538658220b5aaf3b6ef53758f0a106dc")?;
    ///
    /// assert_eq!(
    ///     commit,
    ///     browser.oid("e24124b")?,
    /// );
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn oid(&self, oid: &str) -> Result<Oid, Error> {
        self.repository.oid(oid)
    }

    /// Get the [`Diff`] between two commits.
    pub fn diff(&self, from: Oid, to: Oid) -> Result<Diff, Error> {
        self.repository.diff(from, to)
    }

    /// Get the [`Diff`] of a commit with no parents.
    pub fn initial_diff(&self, oid: Oid) -> Result<Diff, Error> {
        self.repository.initial_diff(oid)
    }

    /// List the names of the _branches_ that are contained in the underlying
    /// [`Repository`].
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Branch, RefScope, BranchName, Browser, Namespace, Repository};
    /// use std::convert::TryFrom;
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(&repo, Branch::local("master"))?;
    ///
    /// let branches = browser.list_branches(RefScope::All)?;
    ///
    /// // 'master' exists in the list of branches
    /// assert!(branches.contains(&Branch::local("master")));
    ///
    /// // Filter the branches by `Remote` 'origin'.
    /// let mut branches = browser.list_branches(RefScope::Remote {
    ///     name: Some("origin".to_string())
    /// })?;
    /// branches.sort();
    ///
    /// assert_eq!(branches, vec![
    ///     Branch::remote("HEAD", "origin"),
    ///     Branch::remote("dev", "origin"),
    ///     Branch::remote("master", "origin"),
    /// ]);
    ///
    /// // Filter the branches by all `Remote`s.
    /// let mut branches = browser.list_branches(RefScope::Remote {
    ///     name: None
    /// })?;
    /// branches.sort();
    ///
    /// assert_eq!(branches, vec![
    ///     Branch::remote("HEAD", "origin"),
    ///     Branch::remote("dev", "origin"),
    ///     Branch::remote("master", "origin"),
    ///     Branch::remote("orange/pineapple", "banana"),
    ///     Branch::remote("pineapple", "banana"),
    /// ]);
    ///
    /// // We can also switch namespaces and list the branches in that namespace.
    /// let golden = browser.switch_namespace(&Namespace::try_from("golden")?, Branch::local("master"))?;
    ///
    /// let mut branches = golden.list_branches(RefScope::Local)?;
    /// branches.sort();
    ///
    /// assert_eq!(branches, vec![
    ///     Branch::local("banana"),
    ///     Branch::local("master"),
    /// ]);
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_branches(&self, filter: RefScope) -> Result<Vec<Branch>, Error> {
        self.repository.list_branches(filter)
    }

    /// List the names of the _tags_ that are contained in the underlying
    /// [`Repository`].
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Branch, RefScope, Browser, Namespace, Oid, Repository, Tag, TagName, Author, Time};
    /// use std::convert::TryFrom;
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(&repo, Branch::local("master"))?;
    ///
    /// let tags = browser.list_tags(RefScope::Local)?;
    ///
    /// assert_eq!(
    ///     tags,
    ///     vec![
    ///         Tag::Light {
    ///             id: Oid::from_str("d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3")?,
    ///             name: TagName::new("v0.1.0"),
    ///             remote: None,
    ///         },
    ///         Tag::Light {
    ///             id: Oid::from_str("2429f097664f9af0c5b7b389ab998b2199ffa977")?,
    ///             name: TagName::new("v0.2.0"),
    ///             remote: None,
    ///         },
    ///         Tag::Light {
    ///             id: Oid::from_str("19bec071db6474af89c866a1bd0e4b1ff76e2b97")?,
    ///             name: TagName::new("v0.3.0"),
    ///             remote: None,
    ///         },
    ///         Tag::Light {
    ///             id: Oid::from_str("91b69e00cd8e5a07e20942e9e4457d83ce7a3ff1")?,
    ///             name: TagName::new("v0.4.0"),
    ///             remote: None,
    ///         },
    ///         Tag::Light {
    ///             id: Oid::from_str("80ded66281a4de2889cc07293a8f10947c6d57fe")?,
    ///             name: TagName::new("v0.5.0"),
    ///             remote: None,
    ///         },
    ///         Tag::Annotated {
    ///             id: Oid::from_str("4d1f4af2703074d37cb877f4fdbe36322c8e541d")?,
    ///             target_id: Oid::from_str("d6880352fc7fda8f521ae9b7357668b17bb5bad5")?,
    ///             name: TagName::new("v0.6.0"),
    ///             remote: None,
    ///             tagger: Some(Author {
    ///               name: "Thomas Scholtes".to_string(),
    ///               email: "thomas@monadic.xyz".to_string(),
    ///               time: Time::new(1620740737, 120),
    ///             }),
    ///             message: Some("An annotated tag message for v0.6.0\n".to_string())
    ///         },
    ///     ]
    /// );
    ///
    /// // We can also switch namespaces and list the branches in that namespace.
    /// let golden = browser.switch_namespace(&Namespace::try_from("golden")?, Branch::local("master"))?;
    ///
    /// let branches = golden.list_tags(RefScope::Local)?;
    /// assert_eq!(branches, vec![
    ///     Tag::Light {
    ///         id: Oid::from_str("d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3")?,
    ///         name: TagName::new("v0.1.0"),
    ///         remote: None,
    ///     },
    ///     Tag::Light {
    ///         id: Oid::from_str("2429f097664f9af0c5b7b389ab998b2199ffa977")?,
    ///         name: TagName::new("v0.2.0"),
    ///         remote: None,
    ///     },
    /// ]);
    /// let golden = golden.switch_namespace(&Namespace::try_from("golden")?, Branch::local("master"))?;
    ///
    /// let branches = golden.list_tags(RefScope::Remote { name: Some("kickflip".to_string()) })?;
    /// assert_eq!(branches, vec![
    ///     Tag::Light {
    ///         id: Oid::from_str("d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3")?,
    ///         name: TagName::new("v0.1.0"),
    ///         remote: Some("kickflip".to_string()),
    ///     },
    /// ]);
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_tags(&self, scope: RefScope) -> Result<Vec<Tag>, Error> {
        self.repository.list_tags(scope)
    }

    /// List the namespaces within a `Browser`, filtering out ones that do not
    /// parse correctly.
    ///
    /// # Errors
    ///
    /// * [`Error::Git`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Branch, BranchType, BranchName, Browser, Namespace, Repository};
    /// use std::convert::TryFrom;
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(&repo, Branch::local("master"))?;
    ///
    /// let mut namespaces = browser.list_namespaces()?;
    /// namespaces.sort();
    ///
    /// assert_eq!(namespaces, vec![
    ///     Namespace::try_from("golden")?,
    ///     Namespace::try_from("golden/silver")?,
    ///     Namespace::try_from("me")?,
    /// ]);
    ///
    ///
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_namespaces(&self) -> Result<Vec<Namespace>, Error> {
        self.repository.list_namespaces()
    }

    /// Given a [`crate::file_system::Path`] to a file, return the last
    /// [`Commit`] that touched that file or directory.
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    /// * [`error::Error::LastCommitException`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Branch, Browser, Oid, Repository};
    /// use radicle_surf::file_system::{Label, Path, SystemType};
    /// use radicle_surf::file_system::unsound;
    /// use std::str::FromStr;
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(&repo, Branch::local("master"))?;
    ///
    /// // Clamp the Browser to a particular commit
    /// let commit = Oid::from_str("d6880352fc7fda8f521ae9b7357668b17bb5bad5")?;
    /// browser.commit(commit)?;
    ///
    /// let head_commit = browser.get().first().clone();
    /// let expected_commit = Oid::from_str("d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3")?;
    ///
    /// let readme_last_commit = browser
    ///     .last_commit(Path::with_root(&[unsound::label::new("README.md")]))?
    ///     .map(|commit| commit.id);
    ///
    /// assert_eq!(readme_last_commit, Some(expected_commit));
    ///
    /// let expected_commit = Oid::from_str("e24124b7538658220b5aaf3b6ef53758f0a106dc")?;
    ///
    /// let memory_last_commit = browser
    ///     .last_commit(Path::with_root(&[unsound::label::new("src"), unsound::label::new("memory.rs")]))?
    ///     .map(|commit| commit.id);
    ///
    /// assert_eq!(memory_last_commit, Some(expected_commit));
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn last_commit(&self, path: file_system::Path) -> Result<Option<Commit>, Error> {
        let file_history = self.repository.file_history(
            &path,
            repo::CommitHistory::Last,
            self.get().first().clone(),
        )?;
        Ok(file_history.first().cloned())
    }

    /// Get the commit history for a file _or_ directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use nonempty::NonEmpty;
    /// use radicle_surf::vcs::git::{Branch, Browser, Oid, Repository};
    /// use radicle_surf::file_system::{Label, Path, SystemType};
    /// use radicle_surf::file_system::unsound;
    /// use std::str::FromStr;
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(&repo, Branch::local("master"))?;
    ///
    /// // Clamp the Browser to a particular commit
    /// let commit = Oid::from_str("223aaf87d6ea62eef0014857640fd7c8dd0f80b5")?;
    /// browser.commit(commit)?;
    ///
    /// let root_commits: Vec<Oid> = browser
    ///     .file_history(unsound::path::new("~"))?
    ///     .into_iter()
    ///     .map(|commit| commit.id)
    ///     .collect();
    ///
    /// assert_eq!(root_commits,
    ///     vec![
    ///         Oid::from_str("223aaf87d6ea62eef0014857640fd7c8dd0f80b5")?,
    ///         Oid::from_str("80bacafba303bf0cdf6142921f430ff265f25095")?,
    ///         Oid::from_str("a57846bbc8ced6587bf8329fc4bce970eb7b757e")?,
    ///         Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?,
    ///         Oid::from_str("80ded66281a4de2889cc07293a8f10947c6d57fe")?,
    ///         Oid::from_str("91b69e00cd8e5a07e20942e9e4457d83ce7a3ff1")?,
    ///         Oid::from_str("1820cb07c1a890016ca5578aa652fd4d4c38967e")?,
    ///         Oid::from_str("1e0206da8571ca71c51c91154e2fee376e09b4e7")?,
    ///         Oid::from_str("e24124b7538658220b5aaf3b6ef53758f0a106dc")?,
    ///         Oid::from_str("19bec071db6474af89c866a1bd0e4b1ff76e2b97")?,
    ///         Oid::from_str("f3a089488f4cfd1a240a9c01b3fcc4c34a4e97b2")?,
    ///         Oid::from_str("2429f097664f9af0c5b7b389ab998b2199ffa977")?,
    ///         Oid::from_str("d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3")?,
    ///     ]
    /// );
    ///
    /// let eval_commits: Vec<Oid> = browser
    ///     .file_history(unsound::path::new("~/src/Eval.hs"))?
    ///     .into_iter()
    ///     .map(|commit| commit.id)
    ///     .collect();
    ///
    /// assert_eq!(eval_commits,
    ///     vec![
    ///         Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?,
    ///         Oid::from_str("e24124b7538658220b5aaf3b6ef53758f0a106dc")?,
    ///     ]
    /// );
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn file_history(&self, path: file_system::Path) -> Result<Vec<Commit>, Error> {
        self.repository
            .file_history(&path, repo::CommitHistory::Full, self.get().first().clone())
    }

    /// Extract the signature for a commit
    ///
    /// # Arguments
    ///
    /// * `commit` - The commit to extract the signature for
    /// * `field` - the name of the header field containing the signature block;
    ///   pass `None` to extract the default 'gpgsig'
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Branch, Browser, Repository, Oid, error};
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(&repo, Branch::local("master"))?;
    ///
    /// let commit_with_signature_oid = Oid::from_str(
    ///     "e24124b7538658220b5aaf3b6ef53758f0a106dc"
    /// )?;
    ///
    /// browser.commit(commit_with_signature_oid)?;
    /// let history = browser.get();
    /// let commit_with_signature = history.first();
    /// let signature = browser.extract_signature(commit_with_signature, None)?;
    ///
    /// // We have a signature
    /// assert!(signature.is_some());
    ///
    /// let commit_without_signature_oid = Oid::from_str(
    ///     "80bacafba303bf0cdf6142921f430ff265f25095"
    /// )?;
    ///
    /// browser.commit(commit_without_signature_oid)?;
    /// let history = browser.get();
    /// let commit_without_signature = history.first();
    /// let signature = browser.extract_signature(commit_without_signature, None)?;
    ///
    /// // There is no signature
    /// assert!(signature.is_none());
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_signature(
        &self,
        commit: &Commit,
        field: Option<&str>,
    ) -> Result<Option<Signature>, Error> {
        self.repository.extract_signature(&commit.id, field)
    }

    /// List the [`Branch`]es, which contain the provided [`Commit`].
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Browser, Repository, Branch, BranchName, Namespace, Oid};
    /// use std::convert::TryFrom;
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let browser = Browser::new(&repo, Branch::local("master"))?;
    ///
    ///
    /// let branches = browser.revision_branches(Oid::from_str("27acd68c7504755aa11023300890bb85bbd69d45")?)?;
    /// assert_eq!(
    ///     branches,
    ///     vec![
    ///         Branch::local("dev"),
    ///         Branch::remote("dev", "origin"),
    ///     ]
    /// );
    ///
    /// // TODO(finto): I worry that this test will fail as other branches get added
    /// let mut branches = browser.revision_branches(Oid::from_str("1820cb07c1a890016ca5578aa652fd4d4c38967e")?)?;
    /// branches.sort();
    /// assert_eq!(
    ///     branches,
    ///     vec![
    ///         Branch::remote("HEAD", "origin"),
    ///         Branch::local("dev"),
    ///         Branch::remote("dev", "origin"),
    ///         Branch::local("master"),
    ///         Branch::remote("master", "origin"),
    ///         Branch::remote("orange/pineapple", "banana"),
    ///         Branch::remote("pineapple", "banana"),
    ///     ]
    /// );
    ///
    /// let golden_browser = browser.switch_namespace(&Namespace::try_from("golden")?,
    /// Branch::local("master"))?;
    ///
    /// let branches = golden_browser.revision_branches(Oid::from_str("27acd68c7504755aa11023300890bb85bbd69d45")?)?;
    /// assert_eq!(
    ///     branches,
    ///     vec![
    ///         Branch::local("banana"),
    ///         Branch::remote("fakie/bigspin", "kickflip"),
    ///         Branch::remote("heelflip", "kickflip"),
    ///     ]
    /// );
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn revision_branches(&self, rev: impl Into<Rev>) -> Result<Vec<Branch>, Error> {
        let commit = self.repository.rev_to_commit(&rev.into())?;
        self.repository.revision_branches(&commit.id())
    }

    /// Get the [`Stats`] of the underlying [`Repository`].
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Branch, Browser, Repository};
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(&repo, Branch::local("master"))?;
    ///
    /// let stats = browser.get_stats()?;
    ///
    /// assert_eq!(stats.branches, 2);
    ///
    /// assert_eq!(stats.commits, 15);
    ///
    /// assert_eq!(stats.contributors, 4);
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_stats(&self) -> Result<Stats, Error> {
        let branches = self.list_branches(RefScope::Local)?.len();
        let commits = self.history.len();

        let contributors = self
            .history
            .iter()
            .cloned()
            .map(|commit| (commit.author.name, commit.author.email))
            .collect::<BTreeSet<_>>();

        Ok(Stats {
            branches,
            commits,
            contributors: contributors.len(),
        })
    }

    /// Do a pre-order TreeWalk of the given commit. This turns a Tree
    /// into a HashMap of Paths and a list of Files. We can then turn that
    /// into a Directory.
    fn get_tree(
        repo: &git2::Repository,
        commit: &Commit,
    ) -> Result<HashMap<file_system::Path, NonEmpty<(file_system::Label, directory::File)>>, Error>
    {
        let mut file_paths_or_error: Result<
            HashMap<file_system::Path, NonEmpty<(file_system::Label, directory::File)>>,
            Error,
        > = Ok(HashMap::new());

        let commit = repo.find_commit(commit.id)?;
        let tree = commit.as_object().peel_to_tree()?;

        tree.walk(
            git2::TreeWalkMode::PreOrder,
            |s, entry| match Self::tree_entry_to_file_and_path(repo, s, entry) {
                Ok((path, name, file)) => {
                    match file_paths_or_error.as_mut() {
                        Ok(files) => Self::update_file_map(path, name, file, files),

                        // We don't need to update, we want to keep the error.
                        Err(_err) => {},
                    }
                    git2::TreeWalkResult::Ok
                },
                Err(err) => match err {
                    // We want to continue if the entry was not a Blob.
                    TreeWalkError::NotBlob => git2::TreeWalkResult::Ok,

                    // We found a ObjectType::Commit (likely a submodule) and
                    // so we can skip it.
                    TreeWalkError::Commit => git2::TreeWalkResult::Ok,

                    // But we want to keep the error and abort otherwise.
                    TreeWalkError::Git(err) => {
                        file_paths_or_error = Err(err);
                        git2::TreeWalkResult::Abort
                    },
                },
            },
        )?;

        file_paths_or_error
    }

    /// Find the best common ancestor between two commits if it exists.
    ///
    /// See [`git2::Repository::merge_base`] for details.
    pub fn merge_base(&self, one: Oid, two: Oid) -> Result<Option<Oid>, Error> {
        match self.repository.repo_ref.merge_base(one, two) {
            Ok(merge_base) => Ok(Some(merge_base)),
            Err(err) => {
                if err.code() == git2::ErrorCode::NotFound {
                    Ok(None)
                } else {
                    Err(Error::Git(err))
                }
            },
        }
    }

    fn update_file_map(
        path: file_system::Path,
        name: file_system::Label,
        file: directory::File,
        files: &mut HashMap<file_system::Path, NonEmpty<(file_system::Label, directory::File)>>,
    ) {
        files
            .entry(path)
            .and_modify(|entries| entries.push((name.clone(), file.clone())))
            .or_insert_with(|| NonEmpty::new((name, file)));
    }

    fn tree_entry_to_file_and_path(
        repo: &git2::Repository,
        tree_path: &str,
        entry: &git2::TreeEntry,
    ) -> Result<(file_system::Path, file_system::Label, directory::File), TreeWalkError> {
        // Account for the "root" of git being the empty string
        let path = if tree_path.is_empty() {
            Ok(file_system::Path::root())
        } else {
            file_system::Path::try_from(tree_path)
        }?;

        // We found a Commit object in the Tree, likely a submodule.
        // We will skip this entry.
        if let Some(git2::ObjectType::Commit) = entry.kind() {
            return Err(TreeWalkError::Commit);
        }

        let object = entry.to_object(repo)?;
        let blob = object.as_blob().ok_or(TreeWalkError::NotBlob)?;
        let name = str::from_utf8(entry.name_bytes())?;

        let name = file_system::Label::try_from(name).map_err(Error::FileSystem)?;

        Ok((
            path,
            name,
            directory::File {
                contents: blob.content().to_owned(),
                size: blob.size(),
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "gh-actions"))]
    #[test]
    // An issue with submodules, see: https://github.com/radicle-dev/radicle-surf/issues/54
    fn test_submodule_failure() {
        let repo = Repository::new("..").unwrap();
        let browser = Browser::new(&repo, Branch::local("main")).unwrap();

        browser.get_directory().unwrap();
    }

    #[cfg(test)]
    mod namespace {
        use super::*;
        use pretty_assertions::{assert_eq, assert_ne};

        #[test]
        fn switch_to_banana() -> Result<(), Error> {
            let repo = Repository::new("./data/git-platinum")?;
            let mut browser = Browser::new_with_namespace(
                &repo,
                &Namespace::try_from("golden")?,
                Branch::local("master"),
            )?;
            let history = browser.history.clone();

            browser.branch(Branch::local("banana"))?;

            assert_ne!(history, browser.history);

            Ok(())
        }

        #[test]
        fn me_namespace() -> Result<(), Error> {
            let repo = Repository::new("./data/git-platinum")?;
            let browser = Browser::new(&repo, Branch::local("master"))?;
            let history = browser.history.clone();

            assert_eq!(browser.which_namespace(), Ok(None));

            let browser = browser
                .switch_namespace(&Namespace::try_from("me")?, Branch::local("feature/#1194"))?;

            assert_eq!(
                browser.which_namespace(),
                Ok(Some(Namespace::try_from("me")?))
            );
            assert_eq!(history, browser.history);

            let expected_branches: Vec<Branch> = vec![Branch::local("feature/#1194")];
            let mut branches = browser.list_branches(RefScope::Local)?;
            branches.sort();

            assert_eq!(expected_branches, branches);

            let expected_branches: Vec<Branch> = vec![Branch::remote("feature/#1194", "fein")];
            let mut branches = browser.list_branches(RefScope::Remote {
                name: Some("fein".to_string()),
            })?;
            branches.sort();

            assert_eq!(expected_branches, branches);

            Ok(())
        }

        #[test]
        fn golden_namespace() -> Result<(), Error> {
            let repo = Repository::new("./data/git-platinum")?;
            let browser = Browser::new(&repo, Branch::local("master"))?;
            let history = browser.history.clone();

            assert_eq!(browser.which_namespace(), Ok(None));

            let golden_browser = browser
                .switch_namespace(&Namespace::try_from("golden")?, Branch::local("master"))?;

            assert_eq!(
                golden_browser.which_namespace(),
                Ok(Some(Namespace::try_from("golden")?))
            );
            assert_eq!(history, golden_browser.history);

            let expected_branches: Vec<Branch> =
                vec![Branch::local("banana"), Branch::local("master")];
            let mut branches = golden_browser.list_branches(RefScope::Local)?;
            branches.sort();

            assert_eq!(expected_branches, branches);

            let expected_branches: Vec<Branch> = vec![
                Branch::remote("fakie/bigspin", "kickflip"),
                Branch::remote("heelflip", "kickflip"),
                Branch::remote("v0.1.0", "kickflip"),
            ];
            let mut branches = golden_browser.list_branches(RefScope::Remote {
                name: Some("kickflip".to_string()),
            })?;
            branches.sort();

            assert_eq!(expected_branches, branches);

            Ok(())
        }

        #[test]
        fn silver_namespace() -> Result<(), Error> {
            let repo = Repository::new("./data/git-platinum")?;
            let browser = Browser::new(&repo, Branch::local("master"))?;
            let history = browser.history.clone();

            assert_eq!(browser.which_namespace(), Ok(None));

            let silver_browser = browser.switch_namespace(
                &Namespace::try_from("golden/silver")?,
                Branch::local("master"),
            )?;

            assert_eq!(
                silver_browser.which_namespace(),
                Ok(Some(Namespace::try_from("golden/silver")?))
            );
            assert_ne!(history, silver_browser.history);

            let expected_branches: Vec<Branch> = vec![Branch::local("master")];
            let mut branches = silver_browser.list_branches(RefScope::All)?;
            branches.sort();

            assert_eq!(expected_branches, branches);

            Ok(())
        }
    }

    #[cfg(test)]
    mod rev {
        use super::{Branch, Browser, Error, Oid, Repository, TagName};

        // **FIXME**: This seems to break occasionally on
        // buildkite. For some reason the commit
        // 3873745c8f6ffb45c990eb23b491d4b4b6182f95, which is on master
        // (currently HEAD), is not found. It seems to load the history
        // with d6880352fc7fda8f521ae9b7357668b17bb5bad5 as the HEAD.
        //
        // To temporarily fix this, we need to select "New Build" from the build kite
        // build page that's failing.
        // * Under "Message" put whatever you want.
        // * Under "Branch" put in the branch you're working on.
        // * Expand "Options" and select "clean checkout".
        #[test]
        fn _master() -> Result<(), Error> {
            let repo = Repository::new("./data/git-platinum")?;
            let mut browser = Browser::new(&repo, Branch::local("master"))?;
            browser.rev(Branch::remote("master", "origin"))?;

            let commit1 = Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?;
            assert!(
                browser
                    .history
                    .find(|commit| if commit.id == commit1 {
                        Some(commit.clone())
                    } else {
                        None
                    })
                    .is_some(),
                "commit_id={}, history =\n{:#?}",
                commit1,
                browser.history
            );

            let commit2 = Oid::from_str("d6880352fc7fda8f521ae9b7357668b17bb5bad5")?;
            assert!(
                browser
                    .history
                    .find(|commit| if commit.id == commit2 {
                        Some(commit.clone())
                    } else {
                        None
                    })
                    .is_some(),
                "commit_id={}, history =\n{:#?}",
                commit2,
                browser.history
            );

            Ok(())
        }

        #[test]
        fn commit() -> Result<(), Error> {
            let repo = Repository::new("./data/git-platinum")?;
            let mut browser = Browser::new(&repo, Branch::local("master"))?;
            browser.rev(Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?)?;

            let commit1 = Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?;
            assert!(browser
                .history
                .find(|commit| if commit.id == commit1 {
                    Some(commit.clone())
                } else {
                    None
                })
                .is_some());

            Ok(())
        }

        #[test]
        fn commit_parents() -> Result<(), Error> {
            let repo = Repository::new("./data/git-platinum")?;
            let mut browser = Browser::new(&repo, Branch::local("master"))?;
            browser.rev(Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?)?;
            let commit = browser.history.first();

            assert_eq!(
                commit.parents,
                vec![Oid::from_str("d6880352fc7fda8f521ae9b7357668b17bb5bad5")?]
            );

            Ok(())
        }

        #[test]
        fn commit_short() -> Result<(), Error> {
            let repo = Repository::new("./data/git-platinum")?;
            let mut browser = Browser::new(&repo, Branch::local("master"))?;
            browser.rev(browser.oid("3873745c8")?)?;

            let commit1 = Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?;
            assert!(browser
                .history
                .find(|commit| if commit.id == commit1 {
                    Some(commit.clone())
                } else {
                    None
                })
                .is_some());

            Ok(())
        }

        #[test]
        fn tag() -> Result<(), Error> {
            let repo = Repository::new("./data/git-platinum")?;
            let mut browser = Browser::new(&repo, Branch::local("master"))?;
            browser.rev(TagName::new("v0.2.0"))?;

            let commit1 = Oid::from_str("2429f097664f9af0c5b7b389ab998b2199ffa977")?;
            assert_eq!(browser.history.first().id, commit1);

            Ok(())
        }
    }

    #[cfg(test)]
    mod last_commit {
        use crate::{
            file_system::{unsound, Path},
            vcs::git::{Branch, Browser, Oid, Repository},
        };

        #[test]
        fn readme_missing_and_memory() {
            let repo = Repository::new("./data/git-platinum")
                .expect("Could not retrieve ./data/git-platinum as git repository");
            let mut browser =
                Browser::new(&repo, Branch::local("master")).expect("Could not initialise Browser");

            // Set the browser history to the initial commit
            let commit = Oid::from_str("d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3")
                .expect("Failed to parse SHA");
            browser.commit(commit).unwrap();

            let head_commit = browser.get().0.first().clone();

            // memory.rs is commited later so it should not exist here.
            let memory_last_commit = browser
                .last_commit(Path::with_root(&[
                    unsound::label::new("src"),
                    unsound::label::new("memory.rs"),
                ]))
                .expect("Failed to get last commit")
                .map(|commit| commit.id);

            assert_eq!(memory_last_commit, None);

            // README.md exists in this commit.
            let readme_last_commit = browser
                .last_commit(Path::with_root(&[unsound::label::new("README.md")]))
                .expect("Failed to get last commit")
                .map(|commit| commit.id);

            assert_eq!(readme_last_commit, Some(head_commit.id));
        }

        #[test]
        fn folder_svelte() {
            let repo = Repository::new("./data/git-platinum")
                .expect("Could not retrieve ./data/git-platinum as git repository");
            let mut browser =
                Browser::new(&repo, Branch::local("master")).expect("Could not initialise Browser");

            // Check that last commit is the actual last commit even if head commit differs.
            let commit = Oid::from_str("19bec071db6474af89c866a1bd0e4b1ff76e2b97")
                .expect("Could not parse SHA");
            browser.commit(commit).unwrap();

            let expected_commit_id =
                Oid::from_str("f3a089488f4cfd1a240a9c01b3fcc4c34a4e97b2").unwrap();

            let folder_svelte = browser
                .last_commit(unsound::path::new("~/examples/Folder.svelte"))
                .expect("Failed to get last commit")
                .map(|commit| commit.id);

            assert_eq!(folder_svelte, Some(expected_commit_id));
        }

        #[test]
        fn nest_directory() {
            let repo = Repository::new("./data/git-platinum")
                .expect("Could not retrieve ./data/git-platinum as git repository");
            let mut browser =
                Browser::new(&repo, Branch::local("master")).expect("Could not initialise Browser");

            // Check that last commit is the actual last commit even if head commit differs.
            let commit = Oid::from_str("19bec071db6474af89c866a1bd0e4b1ff76e2b97")
                .expect("Failed to parse SHA");
            browser.commit(commit).unwrap();

            let expected_commit_id =
                Oid::from_str("2429f097664f9af0c5b7b389ab998b2199ffa977").unwrap();

            let nested_directory_tree_commit_id = browser
                .last_commit(unsound::path::new(
                    "~/this/is/a/really/deeply/nested/directory/tree",
                ))
                .expect("Failed to get last commit")
                .map(|commit| commit.id);

            assert_eq!(nested_directory_tree_commit_id, Some(expected_commit_id));
        }

        #[test]
        #[cfg(not(windows))]
        fn can_get_last_commit_for_special_filenames() {
            let repo = Repository::new("./data/git-platinum")
                .expect("Could not retrieve ./data/git-platinum as git repository");
            let mut browser =
                Browser::new(&repo, Branch::local("master")).expect("Could not initialise Browser");

            // Check that last commit is the actual last commit even if head commit differs.
            let commit = Oid::from_str("a0dd9122d33dff2a35f564d564db127152c88e02")
                .expect("Failed to parse SHA");
            browser.commit(commit).unwrap();

            let expected_commit_id =
                Oid::from_str("a0dd9122d33dff2a35f564d564db127152c88e02").unwrap();

            let backslash_commit_id = browser
                .last_commit(unsound::path::new("~/special/faux\\path"))
                .expect("Failed to get last commit")
                .map(|commit| commit.id);
            assert_eq!(backslash_commit_id, Some(expected_commit_id));

            let ogre_commit_id = browser
                .last_commit(unsound::path::new("~/special/👹👹👹"))
                .expect("Failed to get last commit")
                .map(|commit| commit.id);
            assert_eq!(ogre_commit_id, Some(expected_commit_id));
        }

        #[test]
        fn root() {
            let repo = Repository::new("./data/git-platinum")
                .expect("Could not retrieve ./data/git-platinum as git repository");
            let browser =
                Browser::new(&repo, Branch::local("master")).expect("Could not initialise Browser");

            let root_last_commit_id = browser
                .last_commit(Path::root())
                .expect("Failed to get last commit")
                .map(|commit| commit.id);

            assert_eq!(root_last_commit_id, Some(browser.get().first().id));
        }
    }

    #[cfg(test)]
    mod diff {
        use crate::{diff::*, vcs::git::*};

        #[test]
        fn test_initial_diff() -> Result<(), Error> {
            use file_system::*;
            use pretty_assertions::assert_eq;

            let oid = Oid::from_str("d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3")?;
            let repo = Repository::new("./data/git-platinum")?;
            let commit = repo.0.find_commit(oid).unwrap();

            assert!(commit.parents().count() == 0);
            assert!(commit.parent(0).is_err());

            let bro = Browser::new(&repo, Branch::local("master"))?;
            let diff = bro.initial_diff(oid)?;

            let expected_diff = Diff {
                created: vec![CreateFile {
                    path: Path::with_root(&[unsound::label::new("README.md")]),
                    diff: FileDiff::Plain {
                        hunks: vec![Hunk {
                            header: Line(b"@@ -0,0 +1 @@\n".to_vec()),
                            lines: vec![
                                LineDiff::addition(b"This repository is a data source for the Upstream front-end tests.\n".to_vec(), 1),
                            ]
                        }].into()
                    },
                }],
                deleted: vec![],
                moved: vec![],
                copied: vec![],
                modified: vec![],
            };
            assert_eq!(expected_diff, diff);

            Ok(())
        }

        #[test]
        fn test_diff() -> Result<(), Error> {
            use file_system::*;
            use pretty_assertions::assert_eq;

            let repo = Repository::new("./data/git-platinum")?;
            let commit = repo
                .0
                .find_commit(Oid::from_str("80bacafba303bf0cdf6142921f430ff265f25095")?)
                .unwrap();
            let parent = commit.parent(0)?;

            let bro = Browser::new(&repo, Branch::local("master"))?;

            let diff = bro.diff(parent.id(), commit.id())?;

            let expected_diff = Diff {
                created: vec![],
                deleted: vec![],
                moved: vec![],
                copied: vec![],
                modified: vec![ModifiedFile {
                    path: Path::with_root(&[unsound::label::new("README.md")]),
                    diff: FileDiff::Plain {
                        hunks: vec![Hunk {
                            header: Line(b"@@ -1 +1,2 @@\n".to_vec()),
                            lines: vec![
                                LineDiff::deletion(b"This repository is a data source for the Upstream front-end tests.\n".to_vec(), 1),
                                LineDiff::addition(b"This repository is a data source for the Upstream front-end tests and the\n".to_vec(), 1),
                                LineDiff::addition(b"[`radicle-surf`](https://github.com/radicle-dev/git-platinum) unit tests.\n".to_vec(), 2),
                            ]
                        }].into()
                    },
                    eof: None,
                }]
            };
            assert_eq!(expected_diff, diff);

            Ok(())
        }

        #[cfg(feature = "serialize")]
        #[test]
        fn test_diff_serde() -> Result<(), Error> {
            use file_system::*;

            let diff = Diff {
                created: vec![CreateFile{path: unsound::path::new("LICENSE"), diff: FileDiff::Plain { hunks: Hunks::default() }}],
                deleted: vec![],
                moved: vec![
                    MoveFile {
                        old_path: unsound::path::new("CONTRIBUTING"),
                        new_path: unsound::path::new("CONTRIBUTING.md")
                    }
                ],
                copied: vec![],
                modified: vec![ModifiedFile {
                    path: Path::with_root(&[unsound::label::new("README.md")]),
                    diff: FileDiff::Plain {
                        hunks: vec![Hunk {
                            header: Line(b"@@ -1 +1,2 @@\n".to_vec()),
                            lines: vec![
                                LineDiff::deletion(b"This repository is a data source for the Upstream front-end tests.\n".to_vec(), 1),
                                LineDiff::addition(b"This repository is a data source for the Upstream front-end tests and the\n".to_vec(), 1),
                                LineDiff::addition(b"[`radicle-surf`](https://github.com/radicle-dev/git-platinum) unit tests.\n".to_vec(), 2),
                                LineDiff::context(b"\n".to_vec(), 3, 4),
                            ]
                        }].into()
                    },
                    eof: None,
                }]
            };

            let eof: Option<u8> = None;
            let json = serde_json::json!({
                "created": [{"path": "LICENSE", "diff": {
                        "type": "plain",
                        "hunks": []
                    },
                }],
                "deleted": [],
                "moved": [{ "oldPath": "CONTRIBUTING", "newPath": "CONTRIBUTING.md" }],
                "copied": [],
                "modified": [{
                    "path": "README.md",
                    "diff": {
                        "type": "plain",
                        "hunks": [{
                            "header": "@@ -1 +1,2 @@\n",
                            "lines": [
                                { "lineNum": 1,
                                  "line": "This repository is a data source for the Upstream front-end tests.\n",
                                  "type": "deletion"
                                },
                                { "lineNum": 1,
                                  "line": "This repository is a data source for the Upstream front-end tests and the\n",
                                  "type": "addition"
                                },
                                { "lineNum": 2,
                                  "line": "[`radicle-surf`](https://github.com/radicle-dev/git-platinum) unit tests.\n",
                                  "type": "addition"
                                },
                                { "lineNumOld": 3, "lineNumNew": 4,
                                  "line": "\n",
                                  "type": "context"
                                }
                            ]
                        }]
                    },
                    "eof" : eof,
                }]
            });
            assert_eq!(serde_json::to_value(&diff).unwrap(), json);

            Ok(())
        }
    }

    #[cfg(test)]
    mod threading {
        use crate::vcs::git::*;
        use pretty_assertions::assert_eq;
        use std::sync::{Mutex, MutexGuard};

        #[test]
        fn basic_test() -> Result<(), Error> {
            let shared_repo = Mutex::new(Repository::new("./data/git-platinum")?);
            let locked_repo: MutexGuard<Repository> = shared_repo.lock().unwrap();
            let bro = Browser::new(&*locked_repo, Branch::local("master"))?;
            let mut branches = bro.list_branches(RefScope::All)?;
            branches.sort();

            assert_eq!(
                branches,
                vec![
                    Branch::remote("HEAD", "origin"),
                    Branch::remote("dev", "origin"),
                    Branch::local("dev"),
                    Branch::remote("master", "origin"),
                    Branch::local("master"),
                    Branch::remote("orange/pineapple", "banana"),
                    Branch::remote("pineapple", "banana"),
                ]
            );

            Ok(())
        }
    }
}
