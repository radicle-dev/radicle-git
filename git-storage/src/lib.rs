// Copyright © 2022 The Radicle Link Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! # `git-storage`
//!
//! This crate provides access to git [references][refs] and [objects][objs].
//!
//! To first initialise the storage use [`Write::open`].
//!
//! After the storage is initialised, use [`Write::open`] or [`Read::open`] for
//! read-write or read-only access to the underlying storage. These structs will
//! implement the traits below depending on their access levels.
//!
//! ## Read-only access
//!
//! * [`refdb::Read`]
//! * [`odb::Read`]
//!
//! ## Read-write access
//!
//! * [`refdb::Read`]
//! * [`refdb::Write`]
//! * [`odb::Read`]
//! * [`odb::Write`]
//!
//! ## Concurrency
//!
//!
//! [`Read`] and [`Write`] can be sent between threads, but it can't be shared
//! between threads. _Some_ operations are safe to perform concurrently in much
//! the same way two `git` processes can access the same repository.
//! However, if you need multiple [`Read`]/[`Write`]s to be shared between
//! threads, use a [`Pool`] instead.
//!
//! [refs]: https://git-scm.com/book/en/v2/Git-Internals-Git-References
//! [objs]: https://git-scm.com/book/en/v2/Git-Internals-Git-Objects

#[macro_use]
extern crate async_trait;

extern crate radicle_git_ext as git_ext;
extern crate radicle_std_ext as std_ext;

pub mod glob;

pub mod pool;
pub use pool::Pool;

pub mod refdb;
pub use refdb::{Applied, Reference, SymrefTarget, Target, Update, Updated};

pub mod odb;
pub use odb::{Blob, Commit, Object, Tag, Tree};

mod backend;
pub use backend::{
    read::{self, Read},
    write::{self, Write},
};

pub mod signature;

/// Initialise the git backend.
///
/// **SHOULD** be called before all accesses to git functionality.
pub fn init() {
    use libc::c_int;
    use libgit2_sys as raw_git;
    use std::sync::Once;

    static INIT: Once = Once::new();

    unsafe {
        INIT.call_once(|| {
            let ret =
                raw_git::git_libgit2_opts(raw_git::GIT_OPT_SET_MWINDOW_FILE_LIMIT as c_int, 256);
            if ret < 0 {
                panic!(
                    "error setting libgit2 option: {}",
                    git2::Error::last_error(ret).unwrap()
                )
            }
        })
    }
}
