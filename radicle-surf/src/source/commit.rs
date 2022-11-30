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

//! Represents a commit.

use std::path::PathBuf;

use git_ref_format::RefString;
#[cfg(feature = "serde")]
use serde::{
    ser::{SerializeStruct as _, Serializer},
    Serialize,
};

use crate::{
    diff,
    git::{self, glob, Glob, Repository},
    source::person::Person,
};

use radicle_git_ext::Oid;

/// Representation of a changeset between two revs.
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Clone)]
pub struct Commit {
    /// The commit header.
    pub header: Header,
    /// The changeset introduced by this commit.
    pub diff: diff::Diff,
    /// The list of branches this commit belongs to.
    pub branches: Vec<RefString>,
}

/// Representation of a code commit.
#[derive(Clone)]
pub struct Header {
    /// Identifier of the commit in the form of a sha1 hash. Often referred to
    /// as oid or object id.
    pub sha1: Oid,
    /// The author of the commit.
    pub author: Person,
    /// The summary of the commit message body.
    pub summary: String,
    /// The entire commit message body.
    pub message: String,
    /// The committer of the commit.
    pub committer: Person,
    /// The recorded time of the committer signature. This is a convenience
    /// alias until we expose the actual author and commiter signatures.
    pub committer_time: git2::Time,
}

impl Header {
    /// Returns the commit description text. This is the text after the one-line
    /// summary.
    #[must_use]
    pub fn description(&self) -> &str {
        self.message
            .strip_prefix(&self.summary)
            .unwrap_or(&self.message)
            .trim()
    }
}

impl From<&git::Commit> for Header {
    fn from(commit: &git::Commit) -> Self {
        Self {
            sha1: commit.id,
            author: Person {
                name: commit.author.name.clone(),
                email: commit.author.email.clone(),
            },
            summary: commit.summary.clone(),
            message: commit.message.clone(),
            committer: Person {
                name: commit.committer.name.clone(),
                email: commit.committer.email.clone(),
            },
            committer_time: commit.committer.time,
        }
    }
}

impl From<git::Commit> for Header {
    fn from(commit: git::Commit) -> Self {
        Self::from(&commit)
    }
}

#[cfg(feature = "serde")]
impl Serialize for Header {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Header", 6)?;
        state.serialize_field("sha1", &self.sha1.to_string())?;
        state.serialize_field("author", &self.author)?;
        state.serialize_field("summary", &self.summary)?;
        state.serialize_field("description", &self.description())?;
        state.serialize_field("committer", &self.committer)?;
        state.serialize_field("committerTime", &self.committer_time.seconds())?;
        state.end()
    }
}

/// A selection of commit headers and their statistics.
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Commits {
    /// The commit headers
    pub headers: Vec<Header>,
    /// The statistics for the commit headers
    pub stats: git::Stats,
}

/// Retrieves a [`Commit`].
///
/// # Errors
///
/// Will return [`Error`] if the project doesn't exist or the surf interaction
/// fails.
pub fn commit<R: git::Revision>(repo: &Repository, rev: R) -> Result<Commit, Error> {
    let commit = repo.commit(rev)?;
    let sha1 = commit.id;
    let header = Header::from(&commit);
    let diff = repo.diff_commit(commit)?;

    let branches = repo
        .revision_branches(&sha1, Glob::all_heads().branches().and(Glob::all_remotes()))?
        .into_iter()
        .map(|b| b.refname().into())
        .collect();

    Ok(Commit {
        header,
        diff,
        branches,
    })
}

/// Retrieves the [`Header`] for the given `sha1`.
///
/// # Errors
///
/// Will return [`Error`] if the project doesn't exist or the surf interaction
/// fails.
pub fn header(repo: &Repository, sha1: Oid) -> Result<Header, Error> {
    let commit = repo.commit(sha1)?;
    Ok(Header::from(&commit))
}

/// Retrieves the [`Commit`] history for the given `revision`.
///
/// # Errors
///
/// Will return [`Error`] if the project doesn't exist or the surf interaction
/// fails.
pub fn commits<R>(repo: &Repository, revision: &R) -> Result<Commits, Error>
where
    R: git::Revision,
{
    let stats = repo.get_commit_stats(revision)?;
    let commits = repo.history(revision)?.collect::<Result<Vec<_>, _>>()?;
    let headers = commits.into_iter().map(Header::from).collect();
    Ok(Commits { headers, stats })
}

/// An error reported by commit API.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error occurred during a git operation.
    #[error(transparent)]
    Git(#[from] git::Error),

    #[error(transparent)]
    Glob(#[from] glob::Error),

    /// Trying to find a file path which could not be found.
    #[error("the path '{0}' was not found")]
    PathNotFound(PathBuf),
}
