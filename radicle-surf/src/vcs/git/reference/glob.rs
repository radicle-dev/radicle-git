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

use crate::{
    git::RefScope,
    vcs::git::{error, repo::RepositoryRef},
};
use either::Either;
use std::fmt::{self, Write as _};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefGlob {
    /// When calling [`RefGlob::references`] this will return the references via
    /// the globs `refs/heads/*` and `refs/remotes/**/*`.
    Branch,
    /// When calling [`RefGlob::references`] this will return the references via
    /// the glob `refs/heads/*`.
    LocalBranch,
    /// When calling [`RefGlob::references`] this will return the references via
    /// either of the following globs:
    ///     * `refs/remotes/**/*`
    ///     * `refs/remotes/{remote}/*`
    RemoteBranch {
        /// If `remote` is `None` then the `**` wildcard will be used, otherwise
        /// the provided remote name will be used.
        remote: Option<String>,
    },
    /// When calling [`RefGlob::references`] this will return the references via
    /// the globs `refs/tags/*` and `refs/remotes/*/tags`
    Tag,
    /// When calling [`RefGlob::references`] this will return the references via
    /// the glob `refs/tags/*`.
    LocalTag,
    /// When calling [`RefGlob::references`] this will return the references via
    /// either of the following globs:
    ///     * `refs/remotes/*/tags/*`
    ///     * `refs/remotes/{remote}/tags/*`
    RemoteTag {
        /// If `remote` is `None` then the `*` wildcard will be used, otherwise
        /// the provided remote name will be used.
        remote: Option<String>,
    },
    /// refs/namespaces/**
    Namespace,
}

/// Iterator chaining multiple [`git2::References`]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct References<'a> {
    inner: Vec<git2::References<'a>>,
}

impl<'a> References<'a> {
    pub fn iter(self) -> impl Iterator<Item = Result<git2::Reference<'a>, git2::Error>> {
        self.inner.into_iter().flatten()
    }
}

impl RefGlob {
    pub fn branch(scope: RefScope) -> Self {
        match scope {
            RefScope::All => Self::Branch,
            RefScope::Local => Self::LocalBranch,
            RefScope::Remote { name } => Self::RemoteBranch { remote: name },
        }
    }

    pub fn tag(scope: RefScope) -> Self {
        match scope {
            RefScope::All => Self::Tag,
            RefScope::Local => Self::LocalTag,
            RefScope::Remote { name } => Self::RemoteTag { remote: name },
        }
    }

    pub fn references<'a>(&self, repo: &RepositoryRef<'a>) -> Result<References<'a>, error::Error> {
        let namespace = repo
            .which_namespace()?
            .map_or(Either::Left(std::iter::empty()), |namespace| {
                Either::Right(namespace.values.into_iter())
            });
        self.with_namespace_glob(namespace, repo)
    }

    fn with_namespace_glob<'a>(
        &self,
        namespace: impl Iterator<Item = String>,
        repo: &RepositoryRef<'a>,
    ) -> Result<References<'a>, error::Error> {
        let mut namespace_glob = "".to_string();
        for n in namespace {
            let _ = write!(namespace_glob, "refs/namespaces/{n}/");
        }

        Ok(match self {
            Self::Branch => {
                let remotes = repo.repo_ref.references_glob(&format!(
                    "{}{}",
                    namespace_glob,
                    Self::RemoteBranch { remote: None }
                ))?;

                let locals = repo.repo_ref.references_glob(&format!(
                    "{}{}",
                    namespace_glob,
                    &Self::LocalBranch
                ))?;
                References {
                    inner: vec![remotes, locals],
                }
            },
            Self::Tag => {
                let remotes = repo.repo_ref.references_glob(&format!(
                    "{}{}",
                    namespace_glob,
                    Self::RemoteTag { remote: None }
                ))?;

                let locals = repo.repo_ref.references_glob(&format!(
                    "{}{}",
                    namespace_glob,
                    &Self::LocalTag
                ))?;
                References {
                    inner: vec![remotes, locals],
                }
            },
            other => References {
                inner: vec![repo
                    .repo_ref
                    .references_glob(&format!("{}{}", namespace_glob, other,))?],
            },
        })
    }
}

impl fmt::Display for RefGlob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LocalBranch => write!(f, "refs/heads/*"),
            Self::RemoteBranch { remote } => {
                write!(f, "refs/remotes/")?;
                match remote {
                    None => write!(f, "**/*"),
                    Some(remote) => write!(f, "{}/*", remote),
                }
            },
            Self::LocalTag => write!(f, "refs/tags/*"),
            Self::RemoteTag { remote } => {
                let remote = match remote {
                    Some(remote) => remote.as_ref(),
                    None => "*",
                };
                write!(f, "refs/remotes/{}/tags/*", remote)
            },
            // Note: the glob below would be used, but libgit doesn't care for union globs.
            // write!(f, "refs/{{remotes/**/*,heads/*}}")
            Self::Branch | Self::Tag => {
                panic!("{}",
                "fatal: `Display` should not be called on `RefGlob::Branch` or `RefGlob::Tag` Since
                this `enum` is private to the repository, it should not be called from the outside.
                Unfortunately, libgit does not support union of globs otherwise this would display
                refs/{remotes/**/*,heads/*}"
            )
            },
            Self::Namespace => write!(f, "refs/namespaces/**"),
        }
    }
}
