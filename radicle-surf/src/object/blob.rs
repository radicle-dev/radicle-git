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

//! Represents git object type 'blob', i.e. actual file contents.
//! See git [doc](https://git-scm.com/book/en/v2/Git-Internals-Git-Objects) for more details.

use std::str;

use radicle_git_ext::Oid;
#[cfg(feature = "serde")]
use serde::{
    ser::{SerializeStruct as _, Serializer},
    Serialize,
};

use crate::object::commit;

/// Represents a git blob object.
pub struct Blob {
    id: Oid,
    content: BlobContent,
    commit: commit::Header,
}

impl Blob {
    /// Returns the [`Blob`] for a file at `revision` under `path`.
    pub(crate) fn new(id: Oid, content: &[u8], commit: commit::Header) -> Self {
        let content = BlobContent::from(content);
        Self {
            id,
            content,
            commit,
        }
    }

    /// Indicates if the content of the [`Blob`] is binary.
    #[must_use]
    pub fn is_binary(&self) -> bool {
        matches!(self.content, BlobContent::Binary(_))
    }

    pub fn object_id(&self) -> Oid {
        self.id
    }

    pub fn content(&self) -> &BlobContent {
        &self.content
    }

    /// Returns the commit that created this blob.
    pub fn commit(&self) -> &commit::Header {
        &self.commit
    }
}

#[cfg(feature = "serde")]
impl Serialize for Blob {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        const FIELDS: usize = 5;
        let mut state = serializer.serialize_struct("Blob", FIELDS)?;
        state.serialize_field("binary", &self.is_binary())?;
        state.serialize_field("content", &self.content)?;
        state.serialize_field("lastCommit", &self.commit)?;
        state.end()
    }
}

/// Variants of blob content.
#[derive(PartialEq, Eq)]
pub enum BlobContent {
    /// Content is plain text and can be passed as a string.
    Plain(String),
    /// Content is binary and needs special treatment.
    Binary(Vec<u8>),
}

impl BlobContent {
    /// Returns the size of this `BlobContent`.
    pub fn size(&self) -> usize {
        match self {
            Self::Plain(content) => content.len(),
            Self::Binary(bytes) => bytes.len(),
        }
    }

    /// Returns the content as bytes.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Plain(content) => content.as_bytes(),
            Self::Binary(bytes) => &bytes[..],
        }
    }
}

#[cfg(feature = "serde")]
impl Serialize for BlobContent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Plain(content) => serializer.serialize_str(content),
            Self::Binary(bytes) => {
                let encoded = base64::encode(bytes);
                serializer.serialize_str(&encoded)
            },
        }
    }
}

impl From<&[u8]> for BlobContent {
    fn from(bytes: &[u8]) -> Self {
        match str::from_utf8(bytes) {
            Ok(utf8) => BlobContent::Plain(utf8.to_owned()),
            Err(_) => BlobContent::Binary(bytes.to_owned()),
        }
    }
}
