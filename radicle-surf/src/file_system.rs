// This file is part of radicle-surf
// <https://github.com/radicle-dev/radicle-surf>
//
// Copyright (C) 2019-2020 The Radicle Team <dev@radicle.xyz>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 or
// later as published by the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! A model of a non-empty directory data structure that can be searched,
//! queried, and rendered. The concept is to represent VCS directory, but is not
//! necessarily tied to one.
//!
//! # Examples
//!
//! ```
//! use nonempty::NonEmpty;
//! use radicle_surf::file_system as fs;
//!
//! // This used for unsafe set up of the directory, but should not be used in production code.
//! use radicle_surf::file_system::unsound;
//!
//! let mut directory = fs::Directory::root();
//!
//! // Set up root files
//! let readme = fs::File::new(b"Radicle Surfing");
//! let cargo = fs::File::new(b"[package]\nname = \"radicle-surf\"");
//! let root_files = NonEmpty::from((
//!     (unsound::label::new("README.md"), readme),
//!     vec![(unsound::label::new("Cargo.toml"), cargo)],
//! ));
//!
//! // Set up src files
//! let lib = fs::File::new(b"pub mod diff;\npub mod file_system;\n pub mod vcs;");
//! let file_system_mod = fs::File::new(b"pub mod directory;\npub mod error;\nmod path;");
//!
//! directory.insert_files(&[], root_files);
//! directory.insert_file(unsound::path::new("src/lib.rs"), lib.clone());
//! directory.insert_file(unsound::path::new("src/file_system/mod.rs"), file_system_mod);
//!
//! // With a directory in place we can begin to operate on it
//! // The first we will do is list what contents are at the root.
//! let root_contents = directory.list_directory();
//!
//! // Checking that we have the correct contents
//! assert_eq!(
//!     root_contents,
//!     vec![
//!         fs::SystemType::file(unsound::label::new("Cargo.toml")),
//!         fs::SystemType::file(unsound::label::new("README.md")),
//!         fs::SystemType::directory(unsound::label::new("src")),
//!     ]
//! );
//!
//! // We can then go down one level to explore sub-directories
//! // Note here that we can use `Path::new`, since there's guranteed to be a `Label`,
//! // although we cheated and created the label unsafely.
//! let src = directory.find_directory(fs::Path::new(unsound::label::new("src")));
//!
//! // Ensure that we found the src directory
//! assert!(src.is_some());
//! let src = src.unwrap();
//!
//! let src_contents = src.list_directory();
//!
//! // Checking we have the correct contents of 'src'
//! assert_eq!(
//!     src_contents,
//!     vec![
//!         fs::SystemType::directory(unsound::label::new("file_system")),
//!         fs::SystemType::file(unsound::label::new("lib.rs")),
//!     ]
//! );
//!
//! // We can dive down to 'file_system' either from the root or src, they should be the same.
//! assert_eq!(
//!     src.find_directory(unsound::path::new("file_system")),
//!     directory.find_directory(unsound::path::new("src/file_system")),
//! );
//!
//! // We can also find files
//! assert_eq!(
//!     src.find_file(unsound::path::new("lib.rs")),
//!     Some(lib)
//! );
//!
//! // From anywhere
//! assert_eq!(
//!     directory.find_file(unsound::path::new("src/file_system/mod.rs")),
//!     src.find_file(unsound::path::new("file_system/mod.rs")),
//! );
//!
//! // And we can also check the size of directories and files
//! assert_eq!(
//!     directory.find_file(unsound::path::new("src/file_system/mod.rs")).map(|f| f.size()),
//!     Some(43),
//! );
//!
//! assert_eq!(
//!     directory.size(),
//!     137,
//! );
//! ```

pub mod directory;
pub use directory::{Directory, Entries, Entry, File, FileContent};
