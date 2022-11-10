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

#[cfg(feature = "serialize")]
use serde::{
    ser::{SerializeStruct as _, Serializer},
    Serialize,
};

use crate::{
    diff,
    file_system,
    git::{self, BranchName, Glob, RepositoryRef},
    person::Person,
    revision::Revision,
};

use radicle_git_ext::Oid;

/// Commit statistics.
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[derive(Clone)]
pub struct Stats {
    /// Additions.
    pub additions: u64,
    /// Deletions.
    pub deletions: u64,
}

/// Representation of a changeset between two revs.
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[derive(Clone)]
pub struct Commit {
    /// The commit header.
    pub header: Header,
    /// The change statistics for this commit.
    pub stats: Stats,
    /// The changeset introduced by this commit.
    pub diff: diff::Diff,
    /// The list of branches this commit belongs to.
    pub branches: Vec<BranchName>,
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

#[cfg(feature = "serialize")]
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
#[cfg_attr(feature = "serialize", derive(Serialize))]
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
pub fn commit<R: git::Revision>(repo: &RepositoryRef, rev: R) -> Result<Commit, Error> {
    let commit = repo.commit(rev)?;
    let sha1 = commit.id;
    let header = Header::from(&commit);
    let diff = repo.diff_from_parent(commit)?;

    let mut deletions = 0;
    let mut additions = 0;

    for file in &diff.modified {
        if let diff::FileDiff::Plain { ref hunks } = file.diff {
            for hunk in hunks.iter() {
                for line in &hunk.lines {
                    match line {
                        diff::LineDiff::Addition { .. } => additions += 1,
                        diff::LineDiff::Deletion { .. } => deletions += 1,
                        _ => {},
                    }
                }
            }
        }
    }

    for file in &diff.created {
        if let diff::FileDiff::Plain { ref hunks } = file.diff {
            for hunk in hunks.iter() {
                for line in &hunk.lines {
                    if let diff::LineDiff::Addition { .. } = line {
                        additions += 1
                    }
                }
            }
        }
    }

    for file in &diff.deleted {
        if let diff::FileDiff::Plain { ref hunks } = file.diff {
            for hunk in hunks.iter() {
                for line in &hunk.lines {
                    if let diff::LineDiff::Deletion { .. } = line {
                        deletions += 1
                    }
                }
            }
        }
    }

    let branches = repo
        .revision_branches(&sha1, &Glob::heads("*")?.and_remotes("*")?)?
        .into_iter()
        .map(|b| b.name)
        .collect();

    Ok(Commit {
        header,
        stats: Stats {
            additions,
            deletions,
        },
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
pub fn header(repo: &RepositoryRef, sha1: Oid) -> Result<Header, Error> {
    let commit = repo.commit(sha1)?;
    Ok(Header::from(&commit))
}

/// Retrieves the [`Commit`] history for the given `revision`.
///
/// # Errors
///
/// Will return [`Error`] if the project doesn't exist or the surf interaction
/// fails.
pub fn commits<P>(
    repo: &RepositoryRef,
    maybe_revision: Option<Revision<P>>,
) -> Result<Commits, Error>
where
    P: ToString,
{
    let rev = match maybe_revision {
        Some(revision) => revision,
        None => Revision::Sha {
            sha: repo.head_oid()?,
        },
    };

    let stats = repo.get_commit_stats(&rev)?;
    let commits: Result<Vec<git::Commit>, git::Error> = repo.history(&rev)?.collect();
    let headers = commits?.iter().map(Header::from).collect();

    Ok(Commits { headers, stats })
}

/// An error reported by commit API.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error occurred during a file system operation.
    #[error(transparent)]
    FileSystem(#[from] file_system::Error),

    /// An error occurred during a git operation.
    #[error(transparent)]
    Git(#[from] git::error::Error),

    /// Trying to find a file path which could not be found.
    #[error("the path '{0}' was not found")]
    PathNotFound(file_system::Path),
}
