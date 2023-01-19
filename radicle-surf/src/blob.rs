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

use radicle_git_ext::Oid;
#[cfg(feature = "serde")]
use serde::{
    ser::{SerializeStruct as _, Serializer},
    Serialize,
};

use crate::Commit;

/// Represents a git blob object.
///
/// The type parameter `T` could be [`BlobRef`] or [`BlobVec`].
pub struct Blob<T> {
    id: Oid,
    is_binary: bool,
    commit: Commit,
    content: T,
}

impl<T> Blob<T> {
    pub fn object_id(&self) -> Oid {
        self.id
    }

    pub fn is_binary(&self) -> bool {
        self.is_binary
    }

    /// Returns the commit that created this blob.
    pub fn commit(&self) -> &Commit {
        &self.commit
    }

    pub fn content(&self) -> &T {
        &self.content
    }
}

impl<'a> Blob<BlobRef<'a>> {
    /// Returns the [`Blob`] wrapping around an underlying `git2::Blob`.
    pub(crate) fn new(id: Oid, git2_blob: git2::Blob<'a>, commit: Commit) -> Self {
        let is_binary = git2_blob.is_binary();
        let content = BlobRef { inner: git2_blob };
        Self {
            id,
            is_binary,
            content,
            commit,
        }
    }

    /// Converts into a `Blob` with owned content bytes.
    pub fn to_owned(&self) -> Blob<BlobVec> {
        Blob::<BlobVec>::new(
            self.id,
            self.content.as_bytes().to_vec(),
            self.commit.clone(),
            self.is_binary,
        )
    }
}

#[cfg(feature = "serde")]
impl<'a> Serialize for Blob<BlobRef<'a>> {
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

/// Represents a blob with borrowed content bytes.
pub struct BlobRef<'a> {
    inner: git2::Blob<'a>,
}

impl<'a> BlobRef<'a> {
    /// Returns the size of the blob content.
    pub fn size(&self) -> usize {
        self.inner.size()
    }

    /// Returns the content as bytes.
    pub fn as_bytes(&self) -> &[u8] {
        self.inner.content()
    }
}

#[cfg(feature = "serde")]
impl<'a> Serialize for BlobRef<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_bytes(self.as_bytes(), serializer)
    }
}

/// Represents a blob with owned content bytes.
pub struct BlobVec {
    inner: Vec<u8>,
}

impl BlobVec {
    pub fn size(&self) -> usize {
        self.inner.len()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.inner.as_ref()
    }
}

impl Blob<BlobVec> {
    pub(crate) fn new(id: Oid, bytes: Vec<u8>, commit: Commit, is_binary: bool) -> Self {
        let content = BlobVec { inner: bytes };
        Self {
            id,
            is_binary,
            content,
            commit,
        }
    }
}

#[cfg(feature = "serde")]
impl Serialize for BlobVec {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_bytes(self.as_bytes(), serializer)
    }
}

#[cfg(feature = "serde")]
impl Serialize for Blob<BlobVec> {
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

/// Common serialization for a `Blob`'s content bytes.
#[cfg(feature = "serde")]
fn serialize_bytes<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match std::str::from_utf8(bytes) {
        Ok(s) => serializer.serialize_str(s),
        Err(_) => {
            let encoded = base64::encode(bytes);
            serializer.serialize_str(&encoded)
        },
    }
}
