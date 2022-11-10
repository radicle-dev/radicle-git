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

use crate::git::Error;
use git_ref_format::refspec::PatternString;
use std::{convert::TryFrom, marker::PhantomData, str};

/// A collection of globs for T (a git reference type).
pub struct Glob<T> {
    globs: Vec<PatternString>,
    glob_type: PhantomData<T>, // To support different methods for different T.
}

impl<T> Glob<T> {
    /// Returns the globs.
    pub fn globs(&self) -> Vec<&str> {
        self.globs.iter().map(|g| g.as_str()).collect()
    }
}

impl<Namespace> Glob<Namespace> {
    /// Creates a `Glob` for namespaces.
    pub fn namespaces(glob: &str) -> Result<Self, Error> {
        let pattern = PatternString::try_from(format!("refs/namespaces/{}", glob))?;
        let globs = vec![pattern];
        Ok(Self {
            globs,
            glob_type: PhantomData,
        })
    }

    /// Adds namespaces patterns to existing `Glob`.
    pub fn and(mut self, glob: &str) -> Result<Self, Error> {
        let pattern = PatternString::try_from(format!("refs/namespaces/{}", glob))?;
        self.globs.push(pattern);
        Ok(self)
    }
}

impl<Tag> Glob<Tag> {
    /// Creates a `Glob` for local tags.
    pub fn tags(glob: &str) -> Result<Self, Error> {
        let pattern = PatternString::try_from(format!("refs/tags/{}", glob))?;
        let globs = vec![pattern];
        Ok(Self {
            globs,
            glob_type: PhantomData,
        })
    }

    /// Updates a `Glob` to include other tags.
    pub fn and_tags(mut self, glob: &str) -> Result<Self, Error> {
        let pattern = PatternString::try_from(format!("refs/tags/{}", glob))?;
        self.globs.push(pattern);
        Ok(self)
    }
}

impl<Branch> Glob<Branch> {
    /// Creates a `Glob` for local branches.
    pub fn heads(glob: &str) -> Result<Self, Error> {
        let pattern = PatternString::try_from(format!("refs/heads/{}", glob))?;
        let globs = vec![pattern];
        Ok(Self {
            globs,
            glob_type: PhantomData,
        })
    }

    /// Creates a `Glob` for remote branches.
    pub fn remotes(glob: &str) -> Result<Self, Error> {
        let pattern = PatternString::try_from(format!("refs/remotes/{}", glob))?;
        let globs = vec![pattern];
        Ok(Self {
            globs,
            glob_type: PhantomData,
        })
    }

    /// Updates a `Glob` to include local branches.
    pub fn and_heads(mut self, glob: &str) -> Result<Self, Error> {
        let pattern = PatternString::try_from(format!("refs/heads/{}", glob))?;
        self.globs.push(pattern);
        Ok(self)
    }

    /// Updates a `Glob` to include remote branches.
    pub fn and_remotes(mut self, glob: &str) -> Result<Self, Error> {
        let pattern = PatternString::try_from(format!("refs/remotes/{}", glob))?;
        self.globs.push(pattern);
        Ok(self)
    }
}
