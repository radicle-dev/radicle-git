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

//! Errors that can occur within the file system logic.
//!
//! These errors occur due to [`Label`](super::path::Label) and
//! [`Path`](super::path::Path) parsing when using their respective `TryFrom`
//! instances.

use std::ffi::OsStr;
use thiserror::Error;

pub(crate) const EMPTY_PATH: Error = Error::Path(PathError::Empty);
pub(crate) const EMPTY_LABEL: Error = Error::Label(LabelError::Empty);

/// Build an [`Error::Label(LabelError::InvalidUTF8)`] from an
/// [`OsStr`](std::ffi::OsStr)
pub(crate) fn label_invalid_utf8(item: &OsStr) -> Error {
    Error::Label(LabelError::InvalidUTF8 {
        label: item.to_string_lossy().into(),
    })
}

/// Build an [`Error::Label(LabelError::ContainsSlash)`] from a [`str`]
pub(crate) fn label_has_slash(item: &str) -> Error {
    Error::Label(LabelError::ContainsSlash { label: item.into() })
}

/// Error type for all file system errors that can occur.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum Error {
    /// A `LabelError` specific error for parsing a
    /// [`Path`](super::path::Label).
    #[error(transparent)]
    Label(#[from] LabelError),
    /// A `PathError` specific error for parsing a [`Path`](super::path::Path).
    #[error(transparent)]
    Path(#[from] PathError),
}

/// Parse errors for when parsing a string to a [`Path`](super::path::Path).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum PathError {
    /// An error signifying that a [`Path`](super::path::Path) is empty.
    #[error("path is empty")]
    Empty,
}

/// Parse errors for when parsing a string to a [`Label`](super::path::Label).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum LabelError {
    /// An error signifying that a [`Label`](super::path::Label) is contains
    /// invalid UTF-8.
    #[error("label '{label}' contains invalid UTF-8")]
    InvalidUTF8 { label: String },
    /// An error signifying that a [`Label`](super::path::Label) contains a `/`.
    #[error("label '{label}' contains a slash")]
    ContainsSlash { label: String },
    /// An error signifying that a [`Label`](super::path::Label) is empty.
    #[error("label is empty")]
    Empty,
}
