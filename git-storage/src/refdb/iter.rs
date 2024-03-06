// Copyright © 2022 The Radicle Link Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Iterator adaptors over [`git2::References`].

use std::fmt::Debug;

use crate::glob;

use super::{error, Reference};

/// Iterator for [`Reference`]s where the inner [`glob::Pattern`] supplied
/// filters out any non-matching reference names.
pub struct References<'a> {
    pub(crate) inner: ReferencesGlob<'a, glob::RefspecMatcher>,
}

impl<'a> Iterator for References<'a> {
    type Item = Result<Reference, error::Iter>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

pub struct ReferencesGlob<'a, G: glob::Pattern + Debug> {
    pub(crate) iter: git2::References<'a>,
    pub(crate) glob: G,
}

impl<'a, G: glob::Pattern + Debug> Iterator for ReferencesGlob<'a, G> {
    type Item = Result<Reference, error::Iter>;

    fn next(&mut self) -> Option<Self::Item> {
        for reference in &mut self.iter {
            match reference {
                Ok(reference) => match reference.name() {
                    Some(name) if self.glob.matches(name) => {
                        return Some(Reference::try_from(reference).map_err(error::Iter::from))
                    }
                    _ => continue,
                },

                Err(e) => return Some(Err(e.into())),
            }
        }
        None
    }
}
