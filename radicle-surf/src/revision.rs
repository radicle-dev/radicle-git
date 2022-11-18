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

//! Represents revisions

use git_ref_format::{lit, Qualified, RefString};
use nonempty::NonEmpty;

#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};

use radicle_git_ext::Oid;

use crate::git::{self, Error, Glob, RepositoryRef};

/// Types of a peer.
pub enum Category<P, U> {
    /// Local peer.
    Local {
        /// Peer Id
        peer_id: P,
        /// User name
        user: U,
    },
    /// Remote peer.
    Remote {
        /// Peer Id
        peer_id: P,
        /// User name
        user: U,
    },
}

/// A revision selector for a `Browser`.
#[cfg_attr(
    feature = "serialize",
    derive(Deserialize, Serialize),
    serde(rename_all = "camelCase", tag = "type")
)]
#[derive(Debug, Clone)]
pub enum Revision {
    /// Select a tag under the name provided.
    #[cfg_attr(feature = "serialize", serde(rename_all = "camelCase"))]
    Tag {
        /// Name of the tag.
        name: RefString,
    },
    /// Select a branch under the name provided.
    #[cfg_attr(feature = "serialize", serde(rename_all = "camelCase"))]
    Branch {
        /// Name of the branch.
        name: RefString,
        /// The remote peer, if specified.
        remote: Option<RefString>,
    },
    /// Select a SHA1 under the name provided.
    #[cfg_attr(feature = "serialize", serde(rename_all = "camelCase"))]
    Sha {
        /// The SHA1 value.
        sha: Oid,
    },
}

impl git::Revision for &Revision {
    type Error = git2::Error;

    fn object_id(&self, repo: &RepositoryRef) -> Result<Oid, Self::Error> {
        match self {
            Revision::Tag { name } => match name.qualified() {
                None => Qualified::from(lit::refs_tags(name)).object_id(repo),
                Some(name) => name.object_id(repo),
            },
            Revision::Branch { name, remote } => match remote {
                Some(remote) => {
                    Qualified::from(lit::refs_remotes(remote.join(name))).object_id(repo)
                },
                None => git::Branch::local(name).refname().object_id(repo),
            },
            Revision::Sha { sha } => Ok(*sha),
        }
    }
}

/// Bundled response to retrieve both branches and tags for
/// a user's repo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Revisions<P, U> {
    /// The peer peer_id for the user.
    pub peer_id: P,
    /// The user who owns these revisions.
    pub user: U,
    /// List of branch reference names.
    pub branches: NonEmpty<RefString>,
    /// List of tag reference names.
    pub tags: Vec<RefString>,
}

/// Provide the [`Revisions`] for the given `peer_id`, looking for the
/// remote branches.
///
/// If there are no branches then this returns `None`.
///
/// # Errors
///
///   * If we cannot get the branches from the `Browser`
pub fn remote<P, U>(
    repo: &RepositoryRef,
    peer_id: P,
    user: U,
) -> Result<Option<Revisions<P, U>>, Error>
where
    P: Clone + ToString,
{
    let remote_branches =
        repo.branch_names(&Glob::remotes(&format!("{}/*", peer_id.to_string()))?)?;
    Ok(
        NonEmpty::from_vec(remote_branches).map(|branches| Revisions {
            peer_id,
            user,
            branches,
            // TODO(rudolfs): implement remote peer tags once we decide how
            // https://radicle.community/t/git-tags/214
            tags: vec![],
        }),
    )
}

/// Provide the [`Revisions`] for the given `peer_id`, looking for the
/// local branches.
///
/// If there are no branches then this returns `None`.
///
/// # Errors
///
///   * If we cannot get the branches from the `Browser`
pub fn local<P, U>(
    repo: &RepositoryRef,
    peer_id: P,
    user: U,
) -> Result<Option<Revisions<P, U>>, Error>
where
    P: Clone + ToString,
{
    let local_branches = repo.branch_names(&Glob::heads("*")?)?;
    let tags = repo.tag_names()?;
    Ok(
        NonEmpty::from_vec(local_branches).map(|branches| Revisions {
            peer_id,
            user,
            branches,
            tags,
        }),
    )
}

/// Provide the [`Revisions`] of a peer.
///
/// If the peer is [`Category::Local`], meaning that is the current person doing
/// the browsing and no remote is set for the reference.
///
/// Othewise, the peer is [`Category::Remote`], meaning that we are looking into
/// a remote part of a reference.
///
/// # Errors
///
///   * If we cannot get the branches from the `Browser`
pub fn revisions<P, U>(
    repo: &RepositoryRef,
    peer: Category<P, U>,
) -> Result<Option<Revisions<P, U>>, Error>
where
    P: Clone + ToString,
{
    match peer {
        Category::Local { peer_id, user } => local(repo, peer_id, user),
        Category::Remote { peer_id, user } => remote(repo, peer_id, user),
    }
}
