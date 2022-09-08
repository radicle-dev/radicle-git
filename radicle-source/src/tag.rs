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

use std::fmt;

use serde::Serialize;

use radicle_surf::{git::RefScope, vcs::git::Browser};

use crate::error::Error;

/// Tag name representation.
///
/// We still need full tag support.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Tag(pub(crate) String);

impl From<String> for Tag {
    fn from(name: String) -> Self {
        Self(name)
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Retrieves the list of [`Tag`] for the given project `id`.
///
/// # Errors
///
/// Will return [`Error`] if the project doesn't exist or the surf interaction
/// fails.
pub fn tags(browser: &Browser<'_>) -> Result<Vec<Tag>, Error> {
    let tag_names = browser.list_tags(RefScope::Local)?;
    let mut tags: Vec<Tag> = tag_names
        .into_iter()
        .map(|tag_name| Tag(tag_name.name().to_string()))
        .collect();

    tags.sort();

    Ok(tags)
}
