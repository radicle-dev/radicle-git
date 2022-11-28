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

use std::{path::Path, str};

#[cfg(feature = "serde")]
use serde::{
    ser::{SerializeStruct as _, Serializer},
    Serialize,
};

use crate::{
    file_system::{File, FileContent},
    git::{self, Repository},
    source::{commit, object::Error},
};

/// File data abstraction.
pub struct Blob {
    pub file: File,
    pub content: BlobContent,
    pub commit: Option<commit::Header>,
}

impl Blob {
    /// Returns the [`Blob`] for a file at `revision` under `path`.
    ///
    /// # Errors
    ///
    /// Will return [`Error`] if the project doesn't exist or a surf interaction
    /// fails.
    pub fn new<P, R>(repo: &Repository, revision: &R, path: &P) -> Result<Blob, Error>
    where
        P: AsRef<Path>,
        R: git::Revision,
    {
        Self::make_blob(repo, revision, path, |c| BlobContent::from(c))
    }

    fn make_blob<P, R>(
        repo: &Repository,
        revision: &R,
        path: &P,
        content: impl FnOnce(FileContent) -> BlobContent,
    ) -> Result<Blob, Error>
    where
        P: AsRef<Path>,
        R: git::Revision,
    {
        let path = path.as_ref();
        let root = repo.root_dir(revision)?;

        let file = root
            .find_file(&path, repo)?
            .ok_or_else(|| Error::PathNotFound(path.to_path_buf()))?;

        let last_commit = repo
            .last_commit(path, revision)?
            .map(|c| commit::Header::from(&c));

        let content = content(file.content(repo)?);

        Ok(Blob {
            file,
            content,
            commit: last_commit,
        })
    }

    /// Indicates if the content of the [`Blob`] is binary.
    #[must_use]
    pub fn is_binary(&self) -> bool {
        matches!(self.content, BlobContent::Binary(_))
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
        state.serialize_field("name", &self.file.name())?;
        state.serialize_field("path", &self.file.location())?;
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

impl From<FileContent<'_>> for BlobContent {
    fn from(content: FileContent) -> Self {
        let content = content.as_bytes();
        match str::from_utf8(content) {
            Ok(utf8) => BlobContent::Plain(utf8.to_owned()),
            Err(_) => BlobContent::Binary(content.to_owned()),
        }
    }
}
