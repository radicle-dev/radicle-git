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

use std::{fmt, str};

use thiserror::Error;

use crate::vcs::git::{repo::RepositoryRef, BranchName, Namespace, TagName};
use radicle_git_ext::Oid;
pub(super) mod glob;

/// A revision within the repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rev {
    /// A reference to a branch or tag.
    Ref(Ref),
    /// A particular commit identifier.
    Oid(Oid),
}

impl<R> From<R> for Rev
where
    R: Into<Ref>,
{
    fn from(other: R) -> Self {
        Self::Ref(other.into())
    }
}

impl From<Oid> for Rev {
    fn from(other: Oid) -> Self {
        Self::Oid(other)
    }
}

/// A structured way of referring to a git reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ref {
    /// A git tag, which can be found under `.git/refs/tags/`.
    Tag {
        /// The name of the tag, e.g. `v1.0.0`.
        name: TagName,
    },
    /// A git branch, which can be found under `.git/refs/heads/`.
    LocalBranch {
        /// The name of the branch, e.g. `master`.
        name: BranchName,
    },
    /// A git branch, which can be found under `.git/refs/remotes/`.
    RemoteBranch {
        /// The remote name, e.g. `origin`.
        remote: String,
        /// The name of the branch, e.g. `master`.
        name: BranchName,
    },
    /// A git namespace, which can be found under `.git/refs/namespaces/`.
    ///
    /// Note that namespaces can be nested.
    Namespace {
        /// The name value of the namespace.
        namespace: String,
        /// The reference under that namespace, e.g. The
        /// `refs/remotes/origin/master/ portion of `refs/namespaces/
        /// moi/refs/remotes/origin/master`.
        reference: Box<Ref>,
    },
}

impl Ref {
    /// Add a [`Namespace`] to a `Ref`.
    pub fn namespaced(self, Namespace { values: namespaces }: Namespace) -> Self {
        let mut ref_namespace = self;
        for namespace in namespaces.into_iter().rev() {
            ref_namespace = Self::Namespace {
                namespace,
                reference: Box::new(ref_namespace.clone()),
            };
        }

        ref_namespace
    }

    /// We try to find a [`git2::Reference`] based off of a `Ref` by turning the
    /// ref into a fully qualified ref (e.g. refs/remotes/**/master).
    pub fn find_ref<'a>(
        &self,
        repo: &RepositoryRef<'a>,
    ) -> Result<git2::Reference<'a>, git2::Error> {
        repo.repo_ref.find_reference(&self.to_string())
    }
}

impl fmt::Display for Ref {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tag { name } => write!(f, "refs/tags/{}", name),
            Self::LocalBranch { name } => write!(f, "refs/heads/{}", name),
            Self::RemoteBranch { remote, name } => write!(f, "refs/remotes/{}/{}", remote, name),
            Self::Namespace {
                namespace,
                reference,
            } => write!(f, "refs/namespaces/{}/{}", namespace, reference),
        }
    }
}

/// Error when parsing a ref.
#[derive(Debug, PartialEq, Eq, Error)]
pub enum ParseError {
    /// The parsed ref is malformed.
    #[error("the ref provided '{0}' was malformed")]
    MalformedRef(String),
}

pub mod parser {
    use nom::{bytes, named, tag, IResult};

    use crate::vcs::git::{BranchName, TagName};

    use super::Ref;

    const HEADS: &str = "refs/heads/";
    const REMOTES: &str = "refs/remotes/";
    const TAGS: &str = "refs/tags/";
    const NAMESPACES: &str = "refs/namespaces/";

    named!(heads, tag!(HEADS));
    named!(remotes, tag!(REMOTES));
    named!(tags, tag!(TAGS));
    named!(namsespaces, tag!(NAMESPACES));

    type Error<'a> = nom::Err<nom::error::Error<&'a str>>;

    pub fn component(s: &str) -> IResult<&str, &str> {
        bytes::complete::take_till(|c| c == '/')(s).and_then(|(rest, component)| {
            bytes::complete::take(1u8)(rest).map(|(rest, _)| (rest, component))
        })
    }

    pub fn local(s: &str) -> Result<Ref, Error> {
        bytes::complete::tag(HEADS)(s).map(|(name, _)| Ref::LocalBranch {
            name: BranchName::new(name),
        })
    }

    pub fn remote(s: &str) -> Result<Ref, Error> {
        bytes::complete::tag(REMOTES)(s).and_then(|(rest, _)| {
            component(rest).map(|(rest, remote)| Ref::RemoteBranch {
                remote: remote.to_owned(),
                name: BranchName::new(rest),
            })
        })
    }

    pub fn tag(s: &str) -> Result<Ref, Error> {
        bytes::complete::tag(TAGS)(s).map(|(name, _)| Ref::Tag {
            name: TagName::new(name),
        })
    }

    pub fn namespace(s: &str) -> Result<Ref, Error> {
        bytes::complete::tag(NAMESPACES)(s).and_then(|(rest, _)| {
            component(rest).and_then(|(rest, namespace)| {
                Ok(Ref::Namespace {
                    namespace: namespace.to_owned(),
                    reference: Box::new(parse(rest)?),
                })
            })
        })
    }

    pub fn parse(s: &str) -> Result<Ref, nom::Err<nom::error::Error<&str>>> {
        local(s)
            .or_else(|_| remote(s))
            .or_else(|_| tag(s))
            .or_else(|_| namespace(s))
    }
}

impl str::FromStr for Ref {
    type Err = ParseError;

    fn from_str(reference: &str) -> Result<Self, Self::Err> {
        parser::parse(reference).map_err(|_| ParseError::MalformedRef(reference.to_owned()))
    }
}
