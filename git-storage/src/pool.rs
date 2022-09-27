// Copyright © 2019-2020 The Radicle Foundation <hello@radicle.foundation>
//
// This file is part of radicle-link, distributed under the GPLv3 with Radicle
// Linking Exception. For full terms see the included LICENSE file.

use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::Arc,
};

use deadpool::managed::{self, Manager, Object, RecycleResult};
use parking_lot::RwLock;
use std_ext::Void;
use thiserror::Error;

use crate::{read, signature::UserInfo, write, Read, Write};

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum InitError {
    #[error(transparent)]
    Read(#[from] read::error::Init),

    #[error(transparent)]
    Write(#[from] write::error::Init),
}

pub type Pool<S> = deadpool::managed::Pool<S, InitError>;
pub type PoolError = managed::PoolError<InitError>;

#[async_trait]
pub trait Pooled<S: Send> {
    async fn get(&self) -> Result<PooledRef<S>, PoolError>;
}

#[async_trait]
impl<S: Send> Pooled<S> for Pool<S> {
    async fn get(&self) -> Result<PooledRef<S>, PoolError> {
        self.get().await.map(PooledRef::from)
    }
}

/// A reference to a pooled storage.
///
/// The `S` parameter can be filled by [`Write`] for read-write access or
/// [`Read`] for read-only access.
pub struct PooledRef<S>(Object<S, InitError>);

impl<S> Deref for PooledRef<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<S> DerefMut for PooledRef<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

impl<S> AsRef<S> for PooledRef<S> {
    fn as_ref(&self) -> &S {
        self
    }
}

impl<S> AsMut<S> for PooledRef<S> {
    fn as_mut(&mut self) -> &mut S {
        self
    }
}

impl AsRef<Read> for PooledRef<Write> {
    fn as_ref(&self) -> &Read {
        self.0.read_only()
    }
}

impl<S> From<Object<S, InitError>> for PooledRef<S> {
    fn from(obj: Object<S, InitError>) -> Self {
        Self(obj)
    }
}

#[derive(Clone)]
pub struct Initialised(Arc<RwLock<bool>>);

impl Initialised {
    pub fn no() -> Self {
        Self(Arc::new(RwLock::new(false)))
    }
}

pub struct Writer {
    init: Initialised,
}

#[derive(Clone)]
pub struct Config<W> {
    root: PathBuf,
    info: UserInfo,
    write: W,
}

pub type ReadConfig = Config<PhantomData<Void>>;
pub type WriteConfig = Config<Writer>;

impl ReadConfig {
    pub fn new(root: PathBuf, info: UserInfo) -> Self {
        Config {
            root,
            info,
            write: PhantomData,
        }
    }

    pub fn write(self, init: Initialised) -> WriteConfig {
        Config {
            root: self.root,
            info: self.info,
            write: Writer { init },
        }
    }
}

#[async_trait]
impl Manager<Read, InitError> for ReadConfig {
    async fn create(&self) -> Result<Read, InitError> {
        Read::open(&self.root).map_err(InitError::from)
    }

    async fn recycle(&self, _: &mut Read) -> RecycleResult<InitError> {
        Ok(())
    }
}

impl WriteConfig {
    pub fn new(root: PathBuf, info: UserInfo, init: Initialised) -> Self {
        Self {
            root,
            info,
            write: Writer { init },
        }
    }

    fn mk_storage(&self) -> Result<Write, InitError> {
        Write::open(&self.root, self.info.clone()).map_err(InitError::from)
    }
}

#[async_trait]
impl Manager<Write, InitError> for WriteConfig {
    async fn create(&self) -> Result<Write, InitError> {
        let initialised = self.write.init.0.read();
        if *initialised {
            self.mk_storage()
        } else {
            drop(initialised);
            let mut initialised = self.write.init.0.write();
            self.mk_storage()
                .map(|storage| {
                    *initialised = true;
                    storage
                })
                .map_err(InitError::from)
        }
    }

    async fn recycle(&self, _: &mut Write) -> RecycleResult<InitError> {
        Ok(())
    }
}
