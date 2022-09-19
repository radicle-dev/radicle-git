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

//! Represents a person in a repo.

#[cfg(feature = "serialize")]
use serde::Serialize;

/// Representation of a person (e.g. committer, author, signer) from a
/// repository. Usually extracted from a signature.
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[derive(Clone, Debug)]
pub struct Person {
    /// Name part of the commit signature.
    pub name: String,
    /// Email part of the commit signature.
    pub email: String,
}
