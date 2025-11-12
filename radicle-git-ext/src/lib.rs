//! Extensions and wrappers for `git2` types

pub mod author;
pub mod blob;
pub mod commit;
pub mod error;
pub mod oid;
pub mod revwalk;
pub mod transport;
pub mod tree;

pub use blob::*;
pub use error::*;
pub use oid::*;
pub use revwalk::*;
pub use transport::*;
pub use tree::Tree;

pub use git_ref_format as ref_format;
