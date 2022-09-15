// Copyright © 2022 The Radicle Link Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `odb` is separated into two traits: [`Read`] and [`Write`], providing
//! access to [git objects][objs].
//!
//! The [`Read`] trait provides functions for read-only access to the odb.
//! The [`Write`] trait provides functions for read and write access to the
//! odb, thus it implies the [`Read`] trait.
//!
//! The reason for separating these types of actions out is that one can infer
//! what kind of access a function has to the odb by looking at which trait it
//! is using.
//!
//! For implementations of these traits, this crate provides [`crate::Read`] and
//! [`crate::Write`] structs.
//!
//! [objs]: https://git-scm.com/book/en/v2/Git-Internals-Git-Objects

// TODO: this doesn't abstract over the git2 types very well, but it's too much
// hassle to massage that right now.

use std::error::Error;

pub use git2::{Blob, Commit, Object, Tag, Tree};

use git_ext::Oid;

pub mod read;
pub use read::Read;

pub mod write;
pub use write::Write;
