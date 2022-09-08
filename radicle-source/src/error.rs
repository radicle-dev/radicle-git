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

use radicle_surf::{file_system, git};

/// An error occurred when interacting with [`radicle_surf`] for browsing source
/// code.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// We expect at least one [`crate::revision::Revisions`] when looking at a
    /// project, however the computation found none.
    #[error(
        "while trying to get user revisions we could not find any, there should be at least one"
    )]
    EmptyRevisions,

    /// An error occurred during a [`radicle_surf::file_system`] operation.
    #[error(transparent)]
    FileSystem(#[from] file_system::Error),

    /// An error occurred during a [`radicle_surf::git`] operation.
    #[error(transparent)]
    Git(#[from] git::error::Error),

    /// When trying to query a repositories branches, but there are none.
    #[error("the repository has no branches")]
    NoBranches,

    /// Trying to find a file path which could not be found.
    #[error("the path '{0}' was not found")]
    PathNotFound(file_system::Path),
}
