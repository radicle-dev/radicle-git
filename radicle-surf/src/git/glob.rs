// This file is part of radicle-git
// <https://github.com/radicle-dev/radicle-git>
//
// Copyright (C) 2022 The Radicle Team <dev@radicle.xyz>
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

use std::{convert::TryFrom, marker::PhantomData, str};

use git_ref_format::{
    refname,
    refspec::{PatternString, QualifiedPattern},
    RefString,
};
use thiserror::Error;

use crate::git::{Branch, Namespace, Tag};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    RefFormat(#[from] git_ref_format::Error),
}

/// A collection of globs for T (a git reference type).
pub struct Glob<T> {
    globs: Vec<QualifiedPattern<'static>>,
    glob_type: PhantomData<T>, // To support different methods for different T.
}

impl<T> Glob<T> {
    pub fn globs(&self) -> impl Iterator<Item = &QualifiedPattern<'static>> {
        self.globs.iter()
    }
}

impl Glob<Namespace> {
    /// Creates a `Glob` for namespaces.
    pub fn namespaces(glob: &str) -> Result<Self, Error> {
        let globs = vec![Self::qualify(glob)?];
        Ok(Self {
            globs,
            glob_type: PhantomData,
        })
    }

    /// Adds namespaces patterns to existing `Glob`.
    pub fn and(mut self, glob: &str) -> Result<Self, Error> {
        self.globs.push(Self::qualify(glob)?);
        Ok(self)
    }

    fn qualify(glob: &str) -> Result<QualifiedPattern<'static>, Error> {
        Ok(
            qualify(refname!("refs/namespaces"), PatternString::try_from(glob)?)
                .expect("BUG: pattern is qualified"),
        )
    }
}

impl FromIterator<PatternString> for Glob<Namespace> {
    fn from_iter<T: IntoIterator<Item = PatternString>>(iter: T) -> Self {
        let globs = iter
            .into_iter()
            .map(|pat| {
                qualify(refname!("refs/namespaces"), pat).expect("BUG: pattern is qualified")
            })
            .collect();

        Self {
            globs,
            glob_type: PhantomData,
        }
    }
}

impl Glob<Tag> {
    /// Creates a `Glob` for local tags.
    pub fn tags(glob: &str) -> Result<Self, Error> {
        let pattern = Self::qualify(glob)?;
        let globs = vec![pattern];
        Ok(Self {
            globs,
            glob_type: PhantomData,
        })
    }

    /// Updates a `Glob` to include other tags.
    pub fn and_tags(mut self, glob: &str) -> Result<Self, Error> {
        self.globs.push(Self::qualify(glob)?);
        Ok(self)
    }

    fn qualify(glob: &str) -> Result<QualifiedPattern<'static>, Error> {
        Ok(
            qualify(refname!("refs/tags"), PatternString::try_from(glob)?)
                .expect("BUG: pattern is qualified"),
        )
    }
}

impl FromIterator<PatternString> for Glob<Tag> {
    fn from_iter<T: IntoIterator<Item = PatternString>>(iter: T) -> Self {
        let globs = iter
            .into_iter()
            .map(|pat| qualify(refname!("refs/tags"), pat).expect("BUG: pattern is qualified"))
            .collect();

        Self {
            globs,
            glob_type: PhantomData,
        }
    }
}

impl Glob<Branch> {
    /// Creates a `Glob` for local branches.
    pub fn heads(glob: &str) -> Result<Self, Error> {
        let globs = vec![Self::qualify_heads(glob)?];
        Ok(Self {
            globs,
            glob_type: PhantomData,
        })
    }

    /// Creates a `Glob` for remote branches.
    pub fn remotes(glob: &str) -> Result<Self, Error> {
        let globs = vec![Self::qualify_remotes(glob)?];
        Ok(Self {
            globs,
            glob_type: PhantomData,
        })
    }

    /// Updates a `Glob` to include local branches.
    pub fn and_heads(mut self, glob: &str) -> Result<Self, Error> {
        self.globs.push(Self::qualify_heads(glob)?);
        Ok(self)
    }

    /// Updates a `Glob` to include remote branches.
    pub fn and_remotes(mut self, glob: &str) -> Result<Self, Error> {
        self.globs.push(Self::qualify_remotes(glob)?);
        Ok(self)
    }

    fn qualify_heads(glob: &str) -> Result<QualifiedPattern<'static>, Error> {
        Ok(
            qualify(refname!("refs/heads"), PatternString::try_from(glob)?)
                .expect("BUG: pattern is qualified"),
        )
    }

    fn qualify_remotes(glob: &str) -> Result<QualifiedPattern<'static>, Error> {
        Ok(
            qualify(refname!("refs/remotes"), PatternString::try_from(glob)?)
                .expect("BUG: pattern is qualified"),
        )
    }
}

fn qualify(prefix: RefString, glob: PatternString) -> Option<QualifiedPattern<'static>> {
    prefix.to_pattern(glob).qualified().map(|q| q.into_owned())
}
