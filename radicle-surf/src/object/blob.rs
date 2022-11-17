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

use std::{
    path::{Path, PathBuf},
    str,
};

#[cfg(feature = "serialize")]
use serde::{
    ser::{SerializeStruct as _, Serializer},
    Serialize,
};

use crate::{
    commit,
    git::Repository,
    object::{Error, Info, ObjectType},
    revision::Revision,
};

#[cfg(feature = "syntax")]
use crate::syntax;

/// File data abstraction.
pub struct Blob {
    /// Actual content of the file, if the content is ASCII.
    pub content: BlobContent,
    /// Extra info for the file.
    pub info: Info,
    /// Absolute path to the object from the root of the repo.
    pub path: PathBuf,
}

impl Blob {
    /// Indicates if the content of the [`Blob`] is binary.
    #[must_use]
    pub fn is_binary(&self) -> bool {
        matches!(self.content, BlobContent::Binary(_))
    }

    /// Indicates if the content of the [`Blob`] is HTML.
    #[must_use]
    pub const fn is_html(&self) -> bool {
        matches!(self.content, BlobContent::Html(_))
    }
}

#[cfg(feature = "serialize")]
impl Serialize for Blob {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Blob", 5)?;
        state.serialize_field("binary", &self.is_binary())?;
        state.serialize_field("html", &self.is_html())?;
        state.serialize_field("content", &self.content)?;
        state.serialize_field("info", &self.info)?;
        state.serialize_field("path", &self.path)?;
        state.end()
    }
}

/// Variants of blob content.
#[derive(PartialEq, Eq)]
pub enum BlobContent {
    /// Content is plain text and can be passed as a string.
    Plain(String),
    /// Content is syntax-highlighted HTML.
    ///
    /// Note that is necessary to enable the `syntax` feature flag for this
    /// variant to be constructed. Use `highlighting::blob`, instead of
    /// [`blob`] to get highlighted content.
    Html(String),
    /// Content is binary and needs special treatment.
    Binary(Vec<u8>),
}

#[cfg(feature = "serialize")]
impl Serialize for BlobContent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Plain(content) | Self::Html(content) => serializer.serialize_str(content),
            Self::Binary(bytes) => {
                let encoded = base64::encode(bytes);
                serializer.serialize_str(&encoded)
            },
        }
    }
}

/// Returns the [`Blob`] for a file at `revision` under `path`.
///
/// # Errors
///
/// Will return [`Error`] if the project doesn't exist or a surf interaction
/// fails.
pub fn blob<P>(repo: &Repository, maybe_revision: Option<Revision>, path: &P) -> Result<Blob, Error>
where
    P: AsRef<Path>,
{
    make_blob(repo, maybe_revision, path, content)
}

fn make_blob<P, C>(
    repo: &Repository,
    maybe_revision: Option<Revision>,
    path: &P,
    content: C,
) -> Result<Blob, Error>
where
    P: AsRef<Path>,
    C: FnOnce(&[u8]) -> BlobContent,
{
    let path = path.as_ref();
    let revision = maybe_revision.unwrap();
    let root = repo.root_dir(&revision)?;

    let file = root
        .find_file(&path, repo)?
        .ok_or_else(|| Error::PathNotFound(path.to_path_buf()))?;

    let last_commit = repo
        .last_commit(path, &revision)?
        .map(|c| commit::Header::from(&c));
    // TODO: fuck this
    let name = path
        .file_name()
        .unwrap()
        .to_os_string()
        .into_string()
        .ok()
        .unwrap();

    let file_content = repo.file_content(file)?;
    let content = content(file_content.as_bytes());

    Ok(Blob {
        content,
        info: Info {
            name,
            object_type: ObjectType::Blob,
            last_commit,
        },
        path: path.to_path_buf(),
    })
}

/// Return a [`BlobContent`] given a byte slice.
fn content(content: &[u8]) -> BlobContent {
    match str::from_utf8(content) {
        Ok(utf8) => BlobContent::Plain(utf8.to_owned()),
        Err(_) => BlobContent::Binary(content.to_owned()),
    }
}

#[cfg(feature = "syntax")]
pub mod highlighting {
    use super::*;

    /// Returns the [`Blob`] for a file at `revision` under `path`.
    ///
    /// # Errors
    ///
    /// Will return [`Error`] if the project doesn't exist or a surf interaction
    /// fails.
    pub fn blob<P>(
        browser: &mut Browser,
        maybe_revision: Option<Revision<P>>,
        path: &str,
        theme: Option<&str>,
    ) -> Result<Blob, Error>
    where
        P: ToString,
    {
        make_blob(browser, maybe_revision, path, |contents| {
            content(path, contents, theme)
        })
    }

    /// Return a [`BlobContent`] given a file path, content and theme. Attempts
    /// to perform syntax highlighting when the theme is `Some`.
    fn content(path: &str, content: &[u8], theme_name: Option<&str>) -> BlobContent {
        let content = match str::from_utf8(content) {
            Ok(content) => content,
            Err(_) => return BlobContent::Binary(content.to_owned()),
        };

        match theme_name {
            None => BlobContent::Plain(content.to_owned()),
            Some(theme) => syntax::highlight(path, content, theme)
                .map_or_else(|| BlobContent::Plain(content.to_owned()), BlobContent::Html),
        }
    }
}
