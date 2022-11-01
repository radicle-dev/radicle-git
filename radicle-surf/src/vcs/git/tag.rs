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

use crate::vcs::git::{self, error::Error, reference::Ref, Author};
use radicle_git_ext::Oid;
use std::{convert::TryFrom, fmt, str};

/// A newtype wrapper over `String` to separate out the fact that a caller wants
/// to fetch a tag.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TagName(String);

impl fmt::Display for TagName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl TryFrom<&[u8]> for TagName {
    type Error = str::Utf8Error;

    fn try_from(name: &[u8]) -> Result<Self, Self::Error> {
        let name = str::from_utf8(name)?;
        let short_name = match git::ext::try_extract_refname(name) {
            Ok(stripped) => stripped,
            Err(original) => original,
        };
        Ok(Self(short_name))
    }
}

impl From<TagName> for Ref {
    fn from(other: TagName) -> Self {
        Self::Tag { name: other }
    }
}

impl TagName {
    /// Create a new `TagName`.
    pub fn new(name: &str) -> Self {
        TagName(name.into())
    }

    /// Access the string value of the `TagName`.
    pub fn name(&self) -> &str {
        &self.0
    }
}

/// The static information of a [`git2::Tag`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tag {
    /// A light-weight git tag.
    Light {
        /// The Object ID for the `Tag`, i.e the SHA1 digest.
        id: Oid,
        /// The name that references this `Tag`.
        name: TagName,
        /// If the tag is provided this holds the remote’s name.
        remote: Option<String>,
    },
    /// An annotated git tag.
    Annotated {
        /// The Object ID for the `Tag`, i.e the SHA1 digest.
        id: Oid,
        /// The Object ID for the object that is tagged.
        target_id: Oid,
        /// The name that references this `Tag`.
        name: TagName,
        /// The named author of this `Tag`, if the `Tag` was annotated.
        tagger: Option<Author>,
        /// The message with this `Tag`, if the `Tag` was annotated.
        message: Option<String>,
        /// If the tag is provided this holds the remote’s name.
        remote: Option<String>,
    },
}

impl Tag {
    /// Get the `Oid` of the tag, regardless of its type.
    pub fn id(&self) -> Oid {
        match self {
            Self::Light { id, .. } => *id,
            Self::Annotated { id, .. } => *id,
        }
    }

    /// Get the `TagName` of the tag, regardless of its type.
    pub fn name(&self) -> TagName {
        match self {
            Self::Light { name, .. } => name.clone(),
            Self::Annotated { name, .. } => name.clone(),
        }
    }

    /// Returns the full ref name of the tag.
    pub fn refname(&self) -> String {
        format!("refs/tags/{}", self.name().name())
    }
}

impl<'repo> TryFrom<git2::Tag<'repo>> for Tag {
    type Error = str::Utf8Error;

    fn try_from(tag: git2::Tag) -> Result<Self, Self::Error> {
        let id = tag.id().into();

        let target_id = tag.target_id().into();

        let name = TagName::try_from(tag.name_bytes())?;

        let tagger = tag.tagger().map(Author::try_from).transpose()?;

        let message = tag
            .message_bytes()
            .map(str::from_utf8)
            .transpose()?
            .map(|message| message.into());

        Ok(Tag::Annotated {
            id,
            target_id,
            name,
            tagger,
            message,
            remote: None,
        })
    }
}

impl<'repo> TryFrom<git2::Reference<'repo>> for Tag {
    type Error = Error;

    fn try_from(reference: git2::Reference) -> Result<Self, Self::Error> {
        let name = TagName::try_from(reference.name_bytes())?;

        let (remote, name) = if git::ext::is_remote(&reference) {
            let mut split = name.0.splitn(2, '/');
            let remote = split.next().map(|x| x.to_owned());
            let name = split.next().unwrap();
            (remote, TagName(name.to_owned()))
        } else {
            (None, name)
        };

        match reference.peel_to_tag() {
            Ok(tag) => Ok(Tag::try_from(tag)?),
            Err(err) => {
                // If we get an error peeling to a tag _BUT_ we also have confirmed the
                // reference is a tag, that means we have a lightweight tag,
                // i.e. a commit SHA and name.
                if err.class() == git2::ErrorClass::Object
                    && err.code() == git2::ErrorCode::InvalidSpec
                {
                    let commit = reference.peel_to_commit()?;
                    Ok(Tag::Light {
                        id: commit.id().into(),
                        name,
                        remote,
                    })
                } else {
                    Err(err.into())
                }
            },
        }
    }
}
