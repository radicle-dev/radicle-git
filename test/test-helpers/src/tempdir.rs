use std::{
    io,
    ops::{Deref, DerefMut},
    path::Path,
};

use tempfile::{tempdir, TempDir};

#[derive(Debug)]
pub struct WithTmpDir<A> {
    _tmp: TempDir,
    inner: A,
}

impl<A> WithTmpDir<A> {
    pub fn new<F, E>(mk_inner: F) -> Result<Self, E>
    where
        F: FnOnce(&Path) -> Result<A, E>,
        E: From<io::Error>,
    {
        let tmp = tempdir()?;
        let inner = mk_inner(tmp.path())?;
        Ok(Self { _tmp: tmp, inner })
    }
}

impl<A> Deref for WithTmpDir<A> {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<A> DerefMut for WithTmpDir<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
