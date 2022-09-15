// Copyright © 2019-2020 The Radicle Foundation <hello@radicle.foundation>
//
// This file is part of radicle-link, distributed under the GPLv3 with Radicle
// Linking Exception. For full terms see the included LICENSE file.

//! The [git config][config] for a particular storage.
//!
//! As well as encapsulating the git config, it also holds the [`Owner`] of the
//! config.
//!
//! A [`Config`] can either be constructed by it's [`Config::init`] constuctor
//! or by its `TryFrom` instances for [`git2::Repository`] and [`Write`].
//!
//! [config]: https://git-scm.com/docs/git-config

use std::{convert::TryFrom, marker::PhantomData, path::PathBuf, str::FromStr};

use git_ext::is_not_found_err;
use std_ext::prelude::*;
use thiserror::Error;

use crate::{signature::UserInfo, Write};

const CONFIG_USER_NAME: &str = "user.name";
const CONFIG_USER_EMAIL: &str = "user.email";
const CONFIG_RAD_ID: &str = "rad.identifier";

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("storage was already initialised with identifier {0}")]
    AlreadyInitialised(String),

    #[error("could not parse the identifier found at 'rad.self' in the storage config")]
    Identifier(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),

    #[error("could not parse the identifier found at 'rad.peerid' in the storage config")]
    RadSelf(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),

    #[error(transparent)]
    Git(#[from] git2::Error),
}

/// The `RadIdentifier` is expected to uniquely identify the owner of this
/// repository. The value should be unique and should only be able to be
/// constructed by the owner. For example, this could be a signing key unique to
/// the owner.
pub trait Owner {
    type RadIdentifier: ToString + FromStr + PartialEq;

    fn rad_identifier(&self) -> Self::RadIdentifier;
}

/// The _local_ config for the give [`git2::Repository`].
///
/// This is typically `$GIT_DIR/.git/config` for non-bare, and `$GIT_DIR/config`
/// for bare repositories.
pub fn path(repo: &git2::Repository) -> PathBuf {
    repo.path().join("config")
}

/// A git config paired with the owner information.
///
/// The config is initialised with the following config keys:
///
///   * `user.name`: the provided name of the user.
///   * `user.email`: the provided email of the user. This does not necessarily
///     have to be a valid email, see [`UserInfo`].
///   * `rad.identifier`: the unqiue value identifying the owner of this
///     `Config`.
///
/// # Constructors
///
///   * [`Config::init`]
///   * `TryFrom<&'a Write>`
///   * `TryFrom<git2::Repository`
pub struct Config<'a, O> {
    inner: git2::Config,
    owner: &'a O,
    info: UserInfo,
}

impl<'a, O> TryFrom<&'a Write<O>> for Config<'a, O>
where
    O: Owner,
    <O::RadIdentifier as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    type Error = Error;

    fn try_from(storage: &'a Write<O>) -> Result<Self, Self::Error> {
        let inner = git2::Config::open(&storage.config_path())?;
        let mut this = Self {
            inner,
            owner: storage.owner(),
            info: storage.info().clone(),
        };
        this.guard_key_change()?;
        this.ensure_reflog()?;

        Ok(this)
    }
}

impl TryFrom<&git2::Repository> for Config<'_, PhantomData<Void>> {
    type Error = git2::Error;

    fn try_from(repo: &git2::Repository) -> Result<Self, Self::Error> {
        let inner = git2::Config::open(&self::path(repo))?.snapshot()?;
        let name = inner.get_string(CONFIG_USER_NAME)?;
        let email = inner.get_string(CONFIG_USER_EMAIL)?;
        Ok(Self {
            inner,
            owner: &PhantomData,
            info: UserInfo { name, email },
        })
    }
}

impl<'a, O> Config<'a, O>
where
    O: Owner,
    <O::RadIdentifier as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    fn guard_key_change(&self) -> Result<(), Error> {
        let configured_identifier = self
            .rad_identifier::<O::RadIdentifier>()
            .map(Some)
            .or_matches::<Error, _, _>(
                |err| matches!(err, Error::Git(e) if is_not_found_err(e)),
                || Ok(None),
            )?;
        let owner_identifier = self.owner.rad_identifier();
        match configured_identifier {
            Some(initialised_with) if initialised_with != owner_identifier => {
                Err(Error::AlreadyInitialised(initialised_with.to_string()))
            },

            _ => Ok(()),
        }
    }

    fn ensure_reflog(&mut self) -> Result<(), Error> {
        if let Err(e) = self.inner.get_bool("core.logAllRefUpdates") {
            return if is_not_found_err(&e) {
                Ok(self.inner.set_bool("core.logAllRefUpdates", true)?)
            } else {
                Err(e.into())
            };
        }

        Ok(())
    }

    /// Initialise the [`Config`] with the given `owner` and `info`.
    ///
    /// # Errors
    ///
    ///   * The identifier of the `owner` differs from the existing identifier
    ///     stored
    pub fn init(repo: &mut git2::Repository, owner: &'a O, info: UserInfo) -> Result<Self, Error> {
        let identifier = owner.rad_identifier();
        let config = git2::Config::open(&self::path(repo))?;
        let mut this = Config {
            inner: config,
            owner,
            info,
        };
        this.guard_key_change()?;
        this.ensure_reflog()?;
        this.set_identifier(identifier)?;
        this.set_user_info()?;

        Ok(this)
    }

    fn set_user_info(&mut self) -> Result<(), Error> {
        self.inner.set_str(CONFIG_USER_NAME, &self.info.name)?;
        self.inner.set_str(CONFIG_USER_EMAIL, &self.info.email)?;

        Ok(())
    }

    fn set_identifier(&mut self, rad_identifier: O::RadIdentifier) -> Result<(), Error> {
        self.inner
            .set_str(CONFIG_RAD_ID, &rad_identifier.to_string())
            .map_err(Error::from)
    }
}

impl<O> Config<'_, O> {
    pub fn user(&self) -> &UserInfo {
        &self.info
    }

    pub fn user_name(&self) -> Result<String, Error> {
        self.inner.get_string(CONFIG_USER_NAME).map_err(Error::from)
    }

    pub fn user_email(&self) -> Result<String, Error> {
        self.inner
            .get_string(CONFIG_USER_EMAIL)
            .map_err(Error::from)
    }

    pub fn rad_identifier<Id>(&self) -> Result<Id, Error>
    where
        Id: FromStr,
        Id::Err: std::error::Error + Send + Sync + 'static,
    {
        self.inner
            .get_string(CONFIG_RAD_ID)
            .map_err(Error::from)
            .and_then(|peer_id| {
                peer_id
                    .parse()
                    .map_err(|err| Error::Identifier(Box::new(err)))
            })
    }
}

impl Config<'_, PhantomData<Void>> {
    pub fn readonly(repo: &git2::Repository) -> Result<Self, git2::Error> {
        Self::try_from(repo)
    }
}
