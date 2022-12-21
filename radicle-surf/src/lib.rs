// This file is part of radicle-git
// <https://github.com/radicle-dev/radicle-git>
//
// Copyright (C) 2019-2022 The Radicle Team <dev@radicle.xyz>
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

//! `radicle-surf` is a library to describe a Git repository as a file system.
//! It aims to provide an easy-to-use API to browse a repository via the concept
//! of files and directories for any given revision. It also allows the user to
//! diff any two different revisions.
//!
//! The main entry point of the API is [git::Repository].
//!
//! Let's start surfing!

pub extern crate git_ref_format;

extern crate radicle_git_ext as git_ext;

pub mod diff;
pub mod fs;
pub mod git;
pub mod object;
