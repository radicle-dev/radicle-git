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

use std::convert::TryFrom;

use nonempty::NonEmpty;
use serde::{Deserialize, Serialize};

use radicle_surf::vcs::git::{self, Browser, RefScope, Rev};

use crate::{
    branch::{branches, Branch},
    error::Error,
    oid::Oid,
    tag::{tags, Tag},
};

pub enum Category<P, U> {
    Local { peer_id: P, user: U },
    Remote { peer_id: P, user: U },
}

/// A revision selector for a `Browser`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Revision<P> {
    /// Select a tag under the name provided.
    #[serde(rename_all = "camelCase")]
    Tag {
        /// Name of the tag.
        name: String,
    },
    /// Select a branch under the name provided.
    #[serde(rename_all = "camelCase")]
    Branch {
        /// Name of the branch.
        name: String,
        /// The remote peer, if specified.
        peer_id: Option<P>,
    },
    /// Select a SHA1 under the name provided.
    #[serde(rename_all = "camelCase")]
    Sha {
        /// The SHA1 value.
        sha: Oid,
    },
}

impl<P> TryFrom<Revision<P>> for Rev
where
    P: ToString,
{
    type Error = Error;

    fn try_from(other: Revision<P>) -> Result<Self, Self::Error> {
        match other {
            Revision::Tag { name } => Ok(git::TagName::new(&name).into()),
            Revision::Branch { name, peer_id } => Ok(match peer_id {
                Some(peer) => {
                    git::Branch::remote(&format!("heads/{}", name), &peer.to_string()).into()
                },
                None => git::Branch::local(&name).into(),
            }),
            Revision::Sha { sha } => {
                let oid: git2::Oid = sha.into();
                Ok(oid.into())
            },
        }
    }
}

/// Bundled response to retrieve both [`Branch`]es and [`Tag`]s for a user's
/// repo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Revisions<P, U> {
    /// The peer peer_id for the user.
    pub peer_id: P,
    /// The user who owns these revisions.
    pub user: U,
    /// List of [`git::Branch`].
    pub branches: NonEmpty<Branch>,
    /// List of [`git::Tag`].
    pub tags: Vec<Tag>,
}

/// Provide the [`Revisions`] for the given `peer_id`, looking for the
/// branches as [`RefScope::Remote`].
///
/// If there are no branches then this returns `None`.
///
/// # Errors
///
///   * If we cannot get the branches from the `Browser`
pub fn remote<P, U>(
    browser: &Browser,
    peer_id: P,
    user: U,
) -> Result<Option<Revisions<P, U>>, Error>
where
    P: Clone + ToString,
{
    let remote_branches = branches(browser, Some(peer_id.clone()).into())?;
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
/// branches as [`RefScope::Local`].
///
/// If there are no branches then this returns `None`.
///
/// # Errors
///
///   * If we cannot get the branches from the `Browser`
pub fn local<P, U>(browser: &Browser, peer_id: P, user: U) -> Result<Option<Revisions<P, U>>, Error>
where
    P: Clone + ToString,
{
    let local_branches = branches(browser, RefScope::Local)?;
    let tags = tags(browser)?;
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
    browser: &Browser,
    peer: Category<P, U>,
) -> Result<Option<Revisions<P, U>>, Error>
where
    P: Clone + ToString,
{
    match peer {
        Category::Local { peer_id, user } => local(browser, peer_id, user),
        Category::Remote { peer_id, user } => remote(browser, peer_id, user),
    }
}
