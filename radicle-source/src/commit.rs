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

use std::convert::TryFrom as _;

use serde::{
    ser::{SerializeStruct as _, Serializer},
    Serialize,
};

use radicle_surf::{
    diff,
    vcs::git::{self, Browser, Rev},
};

use crate::{branch::Branch, error::Error, person::Person, revision::Revision};

/// Commit statistics.
#[derive(Clone, Serialize)]
pub struct Stats {
    /// Additions.
    pub additions: u64,
    /// Deletions.
    pub deletions: u64,
}

/// Representation of a changeset between two revs.
#[derive(Clone, Serialize)]
pub struct Commit {
    /// The commit header.
    pub header: Header,
    /// The change statistics for this commit.
    pub stats: Stats,
    /// The changeset introduced by this commit.
    pub diff: diff::Diff,
    /// The list of branches this commit belongs to.
    pub branches: Vec<Branch>,
}

/// Representation of a code commit.
#[derive(Clone)]
pub struct Header {
    /// Identifier of the commit in the form of a sha1 hash. Often referred to
    /// as oid or object id.
    pub sha1: git2::Oid,
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
#[derive(Serialize)]
pub struct Commits {
    /// The commit headers
    pub headers: Vec<Header>,
    /// The statistics for the commit headers
    pub stats: radicle_surf::vcs::git::Stats,
}

/// Retrieves a [`Commit`].
///
/// # Errors
///
/// Will return [`Error`] if the project doesn't exist or the surf interaction
/// fails.
pub fn commit(browser: &mut Browser<'_>, sha1: git2::Oid) -> Result<Commit, Error> {
    browser.commit(sha1)?;

    let history = browser.get();
    let commit = history.first();

    let diff = if let Some(parent) = commit.parents.first() {
        browser.diff(*parent, sha1)?
    } else {
        browser.initial_diff(sha1)?
    };

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

    let branches = browser
        .revision_branches(sha1)?
        .into_iter()
        .map(Branch::from)
        .collect();

    Ok(Commit {
        header: Header::from(commit),
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
pub fn header(browser: &mut Browser<'_>, sha1: git2::Oid) -> Result<Header, Error> {
    browser.commit(sha1)?;

    let history = browser.get();
    let commit = history.first();

    Ok(Header::from(commit))
}

/// Retrieves the [`Commit`] history for the given `revision`.
///
/// # Errors
///
/// Will return [`Error`] if the project doesn't exist or the surf interaction
/// fails.
pub fn commits<P>(
    browser: &mut Browser<'_>,
    maybe_revision: Option<Revision<P>>,
) -> Result<Commits, Error>
where
    P: ToString,
{
    let maybe_revision = maybe_revision.map(Rev::try_from).transpose()?;

    if let Some(revision) = maybe_revision {
        browser.rev(revision)?;
    }

    let headers = browser.get().iter().map(Header::from).collect();
    let stats = browser.get_stats()?;

    Ok(Commits { headers, stats })
}
