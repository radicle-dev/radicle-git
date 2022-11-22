//! The `source` module provides a layer on top of the [`crate::git`]
//! functionality.
//!
//! It provides data types of [`Blob`]s, [`Tree`]s, and [`Commit`]s
//! (see [git objects][git-objects]).  These types are analgous to
//! [`crate::file_system::File`], [`crate::file_system::Directory`], and
//! [`crate::git::Commit`]. However, they provide extra metadata and
//! can be serialized to serve to other applications. For example,
//! they could be used in an HTTP server for viewing a Git repository.
//!
//! [git-objects]: https://git-scm.com/book/en/v2/Git-Internals-Git-Objects

pub mod object;
pub use object::{blob, tree, Blob, BlobContent, Info, ObjectType, Tree};

pub mod commit;
pub use commit::{commit, commits, Commit};

pub mod person;
pub use person::Person;

pub mod revision;
pub use revision::Revision;
