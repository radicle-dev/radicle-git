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

use crate::vcs::git::error::Error;
use git2::Oid;
use std::{convert::TryFrom, str};

/// `Author` is the static information of a [`git2::Signature`].
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Author {
    /// Name of the author.
    pub name: String,
    /// Email of the author.
    pub email: String,
    /// Time the action was taken, e.g. time of commit.
    pub time: git2::Time,
}

impl std::fmt::Debug for Author {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::cmp::Ordering;
        let time = match self.time.offset_minutes().cmp(&0) {
            Ordering::Equal => format!("{}", self.time.seconds()),
            Ordering::Greater => format!("{}+{}", self.time.seconds(), self.time.offset_minutes()),
            Ordering::Less => format!("{}{}", self.time.seconds(), self.time.offset_minutes()),
        };
        f.debug_struct("Author")
            .field("name", &self.name)
            .field("email", &self.email)
            .field("time", &time)
            .finish()
    }
}

impl<'repo> TryFrom<git2::Signature<'repo>> for Author {
    type Error = str::Utf8Error;

    fn try_from(signature: git2::Signature) -> Result<Self, Self::Error> {
        let name = str::from_utf8(signature.name_bytes())?.into();
        let email = str::from_utf8(signature.email_bytes())?.into();
        let time = signature.when();

        Ok(Author { name, email, time })
    }
}

/// `Commit` is the static information of a [`git2::Commit`]. To get back the
/// original `Commit` in the repository we can use the [`Oid`] to retrieve
/// it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Commit {
    /// Object ID of the Commit, i.e. the SHA1 digest.
    pub id: Oid,
    /// The author of the commit.
    pub author: Author,
    /// The actor who committed this commit.
    pub committer: Author,
    /// The long form message of the commit.
    pub message: String,
    /// The summary message of the commit.
    pub summary: String,
    /// The parents of this commit.
    pub parents: Vec<Oid>,
}

impl<'repo> TryFrom<git2::Commit<'repo>> for Commit {
    type Error = Error;

    fn try_from(commit: git2::Commit) -> Result<Self, Self::Error> {
        let id = commit.id();
        let author = Author::try_from(commit.author())?;
        let committer = Author::try_from(commit.committer())?;
        let message_raw = commit.message_bytes();
        let message = str::from_utf8(message_raw)?.into();
        let summary_raw = commit.summary_bytes().ok_or(Error::MissingSummary)?;
        let summary = str::from_utf8(summary_raw)?.into();
        let parents = commit.parent_ids().collect();

        Ok(Commit {
            id,
            author,
            committer,
            message,
            summary,
            parents,
        })
    }
}
