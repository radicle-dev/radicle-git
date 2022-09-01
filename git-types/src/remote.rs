// Copyright © 2019-2020 The Radicle Foundation <hello@radicle.foundation>
//
// This file is part of radicle-link, distributed under the GPLv3 with Radicle
// Linking Exception. For full terms see the included LICENSE file.

use std::{convert::TryFrom, str::FromStr};

use git_ext::{
    error::{is_exists_err, is_not_found_err},
    reference::{self, RefLike},
};
use std_ext::result::ResultExt as _;
use thiserror::Error;

use super::{Fetchspec, Pushspec};

#[derive(Debug, Error)]
pub enum FindError {
    #[error("missing {0}")]
    Missing(&'static str),

    #[error("failed to parse URL")]
    ParseUrl(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),

    #[error("failed to parse refspec")]
    Refspec(#[from] reference::name::Error),

    #[error(transparent)]
    Git(#[from] git2::Error),
}

#[derive(Debug)]
pub struct Remote<Url> {
    /// The file path to the git monorepo.
    pub url: Url,
    /// Name of the remote, e.g. `"rad"`, `"origin"`.
    pub name: RefLike,
    /// The set of fetch specs to add upon creation.
    ///
    /// **Note**: empty fetch specs do not denote the default fetch spec
    /// (`refs/heads/*:refs/remote/<name>/*`), but ... empty fetch specs.
    pub fetchspecs: Vec<Fetchspec>,
    /// The set of push specs to add upon creation.
    pub pushspecs: Vec<Pushspec>,
}

impl<Url> Remote<Url> {
    /// Create a `"rad"` remote with a single fetch spec.
    pub fn rad_remote<Ref, Spec>(url: Url, fetch_spec: Ref) -> Self
    where
        Ref: Into<Option<Spec>>,
        Spec: Into<Fetchspec>,
    {
        Self {
            url,
            name: reflike!("rad"),
            fetchspecs: fetch_spec.into().into_iter().map(Into::into).collect(),
            pushspecs: vec![],
        }
    }

    /// Create a new `Remote` with the given `url` and `name`, while making the
    /// `fetch_spec` and `pushspecs` empty.
    pub fn new<R>(url: Url, name: R) -> Self
    where
        R: Into<RefLike>,
    {
        Self {
            url,
            name: name.into(),
            fetchspecs: vec![],
            pushspecs: vec![],
        }
    }

    /// Override the fetch specs.
    pub fn with_fetchspecs<I>(self, specs: I) -> Self
    where
        I: IntoIterator,
        <I as IntoIterator>::Item: Into<Fetchspec>,
    {
        Self {
            fetchspecs: specs.into_iter().map(Into::into).collect(),
            ..self
        }
    }

    /// Add a fetch spec.
    pub fn add_fetchspec(&mut self, spec: impl Into<Fetchspec>) {
        self.fetchspecs.push(spec.into())
    }

    /// Override the push specs.
    pub fn with_pushspecs<I>(self, specs: I) -> Self
    where
        I: IntoIterator,
        <I as IntoIterator>::Item: Into<Pushspec>,
    {
        Self {
            pushspecs: specs.into_iter().map(Into::into).collect(),
            ..self
        }
    }

    /// Add a push spec.
    pub fn add_pushspec(&mut self, spec: impl Into<Pushspec>) {
        self.pushspecs.push(spec.into())
    }

    /// Persist the remote in the `repo`'s config.
    ///
    /// If a remote with the same name already exists, previous values of the
    /// configuration keys `url`, `fetch`, and `push` will be overwritten.
    /// Note that this means that _other_ configuration keys are left
    /// untouched, if present.
    #[allow(clippy::unit_arg)]
    #[tracing::instrument(skip(self, repo), fields(name = self.name.as_str()))]
    pub fn save(&mut self, repo: &git2::Repository) -> Result<(), git2::Error>
    where
        Url: ToString,
    {
        let url = self.url.to_string();
        repo.remote(self.name.as_str(), &url)
            .and(Ok(()))
            .or_matches::<git2::Error, _, _>(is_exists_err, || Ok(()))?;

        {
            let mut config = repo.config()?;
            config
                .remove_multivar(&format!("remote.{}.url", self.name), ".*")
                .or_matches::<git2::Error, _, _>(is_not_found_err, || Ok(()))?;
            config
                .remove_multivar(&format!("remote.{}.fetch", self.name), ".*")
                .or_matches::<git2::Error, _, _>(is_not_found_err, || Ok(()))?;
            config
                .remove_multivar(&format!("remote.{}.push", self.name), ".*")
                .or_matches::<git2::Error, _, _>(is_not_found_err, || Ok(()))?;
        }

        repo.remote_set_url(self.name.as_str(), &url)?;

        for spec in self.fetchspecs.iter() {
            repo.remote_add_fetch(self.name.as_str(), &spec.to_string())?;
        }
        for spec in self.pushspecs.iter() {
            repo.remote_add_push(self.name.as_str(), &spec.to_string())?;
        }

        debug_assert!(repo.find_remote(self.name.as_str()).is_ok());

        Ok(())
    }

    /// Find a persisted remote by name.
    #[allow(clippy::unit_arg)]
    #[tracing::instrument(skip(repo))]
    pub fn find(repo: &git2::Repository, name: RefLike) -> Result<Option<Self>, FindError>
    where
        Url: FromStr,
        <Url as FromStr>::Err: std::error::Error + Send + Sync + 'static,
    {
        let git_remote = repo
            .find_remote(name.as_str())
            .map(Some)
            .or_matches::<FindError, _, _>(is_not_found_err, || Ok(None))?;

        match git_remote {
            None => Ok(None),
            Some(remote) => {
                let url = remote
                    .url()
                    .ok_or(FindError::Missing("url"))?
                    .parse()
                    .map_err(|e| FindError::ParseUrl(Box::new(e)))?;
                let fetchspecs = remote
                    .fetch_refspecs()?
                    .into_iter()
                    .flatten()
                    .map(Fetchspec::try_from)
                    .collect::<Result<_, _>>()?;
                let pushspecs = remote
                    .push_refspecs()?
                    .into_iter()
                    .flatten()
                    .map(Pushspec::try_from)
                    .collect::<Result<_, _>>()?;

                Ok(Some(Self {
                    url,
                    name,
                    fetchspecs,
                    pushspecs,
                }))
            },
        }
    }
}

impl<Url> AsRef<Url> for Remote<Url> {
    fn as_ref(&self) -> &Url {
        &self.url
    }
}
