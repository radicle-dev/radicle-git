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

use crate::vcs::git::error::Error;
use nonempty::NonEmpty;
pub use radicle_git_ext::Oid;
use std::{convert::TryFrom, fmt, str};

/// A `Namespace` value allows us to switch the git namespace of
/// [`super::Browser`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Namespace {
    /// Since namespaces can be nested we have a vector of strings.
    /// This means that the namespaces `"foo/bar"` is represented as
    /// `vec!["foo", "bar"]`.
    pub(super) values: NonEmpty<String>,
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let values: Vec<_> = self.values.clone().into();
        write!(f, "{}", values.join("/"))
    }
}

impl TryFrom<&str> for Namespace {
    type Error = Error;

    fn try_from(namespace: &str) -> Result<Self, Self::Error> {
        let values = namespace.split('/').map(|n| n.to_string()).collect();
        NonEmpty::from_vec(values)
            .map(|values| Self { values })
            .ok_or(Error::EmptyNamespace)
    }
}

impl TryFrom<&[u8]> for Namespace {
    type Error = Error;

    fn try_from(namespace: &[u8]) -> Result<Self, Self::Error> {
        str::from_utf8(namespace)
            .map_err(Error::from)
            .and_then(Namespace::try_from)
    }
}

impl TryFrom<git2::Reference<'_>> for Namespace {
    type Error = Error;

    fn try_from(reference: git2::Reference) -> Result<Self, Self::Error> {
        let re = regex::Regex::new(r"refs/namespaces/([^/]+)/").unwrap();
        let ref_name = str::from_utf8(reference.name_bytes())?;
        let values = re
            .find_iter(ref_name)
            .map(|m| {
                String::from(
                    m.as_str()
                        .trim_start_matches("refs/namespaces/")
                        .trim_end_matches('/'),
                )
            })
            .collect::<Vec<_>>();

        NonEmpty::from_vec(values)
            .map(|values| Self { values })
            .ok_or(Error::EmptyNamespace)
    }
}
