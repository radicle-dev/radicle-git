// Copyright © 2019-2020 The Radicle Foundation <hello@radicle.foundation>
//
// This file is part of radicle-link, distributed under the GPLv3 with Radicle
// Linking Exception. For full terms see the included LICENSE file.

use std::{fmt, path::Path};

use git_ext::{error::is_not_found_err, ref_format, Oid};
use std_ext::result::ResultExt as _;

use crate::{
    glob,
    odb::{self, Blob, Commit, Object, Tag, Tree},
    refdb::{self, Reference, References, ReferencesGlob},
};

/// A read-only storage backend for accessing git's odb and refdb.
///
/// For access to the odb see [`odb::Read`].
/// For access to the refdb see [`refdb::Read`].
///
/// To construct the `Read` storage use [`Read::open`].
pub struct Read {
    pub(super) raw: git2::Repository,
}

impl Read {
    /// Open the [`Read`] storage using the `path` provided.
    ///
    /// The storage **must** exist already. To initialise the storage for the
    /// first time see [`crate::Write::open`].
    ///
    /// # Concurrency
    ///
    /// [`Read`] can be sent between threads, but it can't be shared between
    /// threads. _Some_ operations are safe to perform concurrently in much
    /// the same way two `git` processes can access the same repository.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, error::Init> {
        crate::init();

        let raw = git2::Repository::open(path)?;

        Ok(Self { raw })
    }
}

impl fmt::Debug for Read {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Read")
            .field("raw", &self.raw.path())
            .finish()
    }
}

pub mod error {
    use thiserror::Error;

    use crate::refdb::error::ParseReference;

    #[derive(Debug, Error)]
    pub enum Init {
        #[error(transparent)]
        Git(#[from] git2::Error),
    }

    #[derive(Debug, Error)]
    pub enum Identifier {
        #[error(transparent)]
        Git(#[from] git2::Error),
    }

    #[derive(Debug, Error)]
    pub enum FindRef {
        #[error(transparent)]
        Git(#[from] git2::Error),
        #[error(transparent)]
        Parse(#[from] ParseReference),
    }
}

// impls

impl<'a> refdb::Read for &'a Read {
    type FindRef = error::FindRef;
    type FindRefs = refdb::error::Iter;
    type FindRefOid = git2::Error;

    type References = References<'a>;

    fn find_reference<Ref>(&self, reference: Ref) -> Result<Option<Reference>, Self::FindRef>
    where
        Ref: AsRef<ref_format::RefStr>,
    {
        let reference = self
            .raw
            .find_reference(reference.as_ref().as_str())
            .map(Some)
            .or_matches::<git2::Error, _, _>(is_not_found_err, || Ok(None))?;
        Ok(reference.map(Reference::try_from).transpose()?)
    }

    fn find_references<Pat>(&self, reference: Pat) -> Result<Self::References, Self::FindRefs>
    where
        Pat: AsRef<ref_format::refspec::PatternStr>,
    {
        Ok(References {
            inner: ReferencesGlob {
                iter: self.raw.references()?,
                glob: glob::RefspecMatcher::from(reference.as_ref().to_owned()),
            },
        })
    }

    fn find_reference_oid<Ref>(&self, reference: Ref) -> Result<Option<Oid>, Self::FindRefOid>
    where
        Ref: AsRef<ref_format::RefStr>,
    {
        self.raw
            .refname_to_id(reference.as_ref().as_str())
            .map(|oid| Some(Oid::from(oid)))
            .or_matches(is_not_found_err, || Ok(None))
    }
}

impl odb::Read for Read {
    type FindObj = git2::Error;
    type FindBlob = git2::Error;
    type FindCommit = git2::Error;
    type FindTag = git2::Error;
    type FindTree = git2::Error;

    fn find_object(&self, oid: Oid) -> Result<Option<Object>, Self::FindObj> {
        self.raw
            .find_object(oid.into(), None)
            .map(Some)
            .or_matches::<git2::Error, _, _>(is_not_found_err, || Ok(None))
    }

    fn find_blob(&self, oid: Oid) -> Result<Option<Blob>, Self::FindBlob> {
        self.raw
            .find_blob(oid.into())
            .map(Some)
            .or_matches(is_not_found_err, || Ok(None))
    }

    fn find_commit(&self, oid: Oid) -> Result<Option<Commit>, Self::FindCommit> {
        self.raw
            .find_commit(oid.into())
            .map(Some)
            .or_matches(is_not_found_err, || Ok(None))
    }

    fn find_tag(&self, oid: Oid) -> Result<Option<Tag>, Self::FindTag> {
        self.raw
            .find_tag(oid.into())
            .map(Some)
            .or_matches(is_not_found_err, || Ok(None))
    }

    fn find_tree(&self, oid: Oid) -> Result<Option<Tree>, Self::FindTree> {
        self.raw
            .find_tree(oid.into())
            .map(Some)
            .or_matches(is_not_found_err, || Ok(None))
    }
}
