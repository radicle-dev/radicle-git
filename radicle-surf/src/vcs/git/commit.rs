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

#[cfg(feature = "serialize")]
use serde::{de, ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};

/// `Author` is the static information of a [`git2::Signature`].
#[cfg_attr(feature = "serialize", derive(Deserialize, Serialize))]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Author {
    /// Name of the author.
    pub name: String,
    /// Email of the author.
    pub email: String,
    /// Time the action was taken, e.g. time of commit.
    #[serde(
        serialize_with = "serialize_time",
        deserialize_with = "deserialize_time"
    )]
    pub time: git2::Time,
}

#[cfg(feature = "serialize")]
fn deserialize_time<'de, D>(deserializer: D) -> Result<git2::Time, D::Error>
where
    D: Deserializer<'de>,
{
    let seconds: i64 = Deserialize::deserialize(deserializer)?;
    Ok(git2::Time::new(seconds, 0))
}

#[cfg(feature = "serialize")]
fn serialize_time<S>(t: &git2::Time, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_i64(t.seconds())
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
#[cfg_attr(feature = "serialize", derive(Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Commit {
    // TODO: Replace git2::Oid with git_ext::Oid (https://github.com/radicle-dev/radicle-git/issues/5)
    /// Object ID of the Commit, i.e. the SHA1 digest.
    #[serde(deserialize_with = "deserialize_oid")]
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
    #[serde(deserialize_with = "deserialize_vec_oid")]
    pub parents: Vec<Oid>,
}

impl Commit {
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

#[cfg(feature = "serialize")]
impl Serialize for Commit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Commit", 7)?;
        state.serialize_field("id", &self.id.to_string())?;
        state.serialize_field("author", &self.author)?;
        state.serialize_field("committer", &self.committer)?;
        state.serialize_field("summary", &self.summary)?;
        state.serialize_field("message", &self.message)?;
        state.serialize_field("description", &self.description())?;
        state.serialize_field(
            "parents",
            &self
                .parents
                .iter()
                .map(|oid| oid.to_string())
                .collect::<Vec<String>>(),
        )?;
        state.end()
    }
}

#[cfg(feature = "serialize")]
fn deserialize_oid<'de, D>(deserializer: D) -> Result<Oid, D::Error>
where
    D: Deserializer<'de>,
{
    let oid: &str = Deserialize::deserialize(deserializer)?;
    Oid::from_str(oid).map_err(|_| {
        serde::de::Error::invalid_type(
            serde::de::Unexpected::Str(oid),
            &"a SHA1 hash not longer than 40 hex characters",
        )
    })
}

#[cfg(feature = "serialize")]
fn deserialize_vec_oid<'de, D>(deserializer: D) -> Result<Vec<Oid>, D::Error>
where
    D: Deserializer<'de>,
{
    let oids: Vec<&str> = Deserialize::deserialize(deserializer)?;
    oids.iter()
        .map(|key| Oid::from_str(key))
        .collect::<Result<Vec<Oid>, _>>()
        .map_err(de::Error::custom)
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

#[cfg(test)]
pub mod tests {
    use git2::Oid;
    use proptest::prelude::*;
    use test_helpers::roundtrip;

    use super::{Author, Commit};

    #[cfg(feature = "serialize")]
    proptest! {
        #[test]
        fn prop_test_commits(commit in commits_strategy()) {
            roundtrip::json(commit)
        }
    }

    fn commits_strategy() -> impl Strategy<Value = Commit> {
        ("[a-fA-F0-9]{40}", any::<String>(), any::<i64>()).prop_map(|(id, text, time)| Commit {
            id: Oid::from_str(&id).unwrap(),
            author: Author {
                name: text.clone(),
                email: text.clone(),
                time: git2::Time::new(time, 0),
            },
            committer: Author {
                name: text.clone(),
                email: text.clone(),
                time: git2::Time::new(time, 0),
            },
            message: text.clone(),
            summary: text,
            parents: vec![Oid::from_str(&id).unwrap(), Oid::from_str(&id).unwrap()],
        })
    }
}
