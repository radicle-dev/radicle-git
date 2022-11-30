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

//! Common definitions for git objects (blob and tree).
//! See git [doc](https://git-scm.com/book/en/v2/Git-Internals-Git-Objects) for more details.

use std::path::PathBuf;

pub mod blob;
pub use blob::{Blob, BlobContent};

pub mod tree;
pub use tree::{Tree, TreeEntry};

use crate::{file_system::directory, git};

/// An error reported by object types.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Directory(#[from] directory::error::Directory),

    #[error(transparent)]
    File(#[from] directory::error::File),

    /// An error occurred during a git operation.
    #[error(transparent)]
    Git(#[from] git::Error),

    /// Trying to find a file path which could not be found.
    #[error("the path '{0}' was not found")]
    PathNotFound(PathBuf),
}
