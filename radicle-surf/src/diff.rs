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

//! Types that represent diff(s) in a Git repo.

#![allow(dead_code, unused_variables, missing_docs)]

use std::path::PathBuf;

#[cfg(feature = "serde")]
use serde::{ser, Serialize, Serializer};

pub mod git;

/// The serializable representation of a `git diff`.
///
/// A [`Diff`] can be retrieved by the following functions:
///    * [`crate::git::Repository::diff`]
///    * [`crate::git::Repository::diff_commit`]
#[cfg_attr(feature = "serde", derive(Serialize), serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Diff {
    pub added: Vec<Added>,
    pub deleted: Vec<Deleted>,
    pub moved: Vec<Moved>,
    pub copied: Vec<Copied>,
    pub modified: Vec<Modified>,
    pub stats: Stats,
}

impl Diff {
    pub fn new() -> Self {
        Diff::default()
    }

    fn modified(
        &mut self,
        path: PathBuf,
        hunks: impl Into<Hunks<Modification>>,
        eof: Option<EofNewLine>,
    ) {
        self.modified.push(Modified {
            path,
            diff: FileDiff::Plain {
                hunks: hunks.into(),
            },
            eof,
        })
    }

    fn moved(&mut self, old_path: PathBuf, new_path: PathBuf) {
        self.moved.push(Moved { old_path, new_path });
    }

    fn copied(&mut self, old_path: PathBuf, new_path: PathBuf) {
        self.copied.push(Copied { old_path, new_path });
    }

    fn modified_binary(&mut self, path: PathBuf) {
        self.modified.push(Modified {
            path,
            diff: FileDiff::Binary,
            eof: None,
        })
    }

    fn added(&mut self, path: PathBuf, diff: FileDiff<Addition>) {
        self.added.push(Added { path, diff })
    }

    fn deleted(&mut self, path: PathBuf, diff: FileDiff<Deletion>) {
        self.deleted.push(Deleted { path, diff })
    }
}

/// A file that was added within a [`Diff`].
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Added {
    /// The path to this file, relative to the repository root.
    pub path: PathBuf,
    /// The set of [`Addition`]s to this file.
    pub diff: FileDiff<Addition>,
}

/// A file that was deleted within a [`Diff`].
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Deleted {
    /// The path to this file, relative to the repository root.
    pub path: PathBuf,
    /// The set of [`Deletion`]s to this file.
    pub diff: FileDiff<Deletion>,
}

/// A file that was moved within a [`Diff`].
#[cfg_attr(feature = "serde", derive(Serialize), serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Moved {
    /// The old path to this file, relative to the repository root.
    pub old_path: PathBuf,
    /// The new path to this file, relative to the repository root.
    pub new_path: PathBuf,
}

/// A file that was copied within a [`Diff`].
#[cfg_attr(feature = "serde", derive(Serialize), serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Copied {
    /// The old path to this file, relative to the repository root.
    pub old_path: PathBuf,
    /// The new path to this file, relative to the repository root.
    pub new_path: PathBuf,
}

#[cfg_attr(feature = "serde", derive(Serialize), serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EofNewLine {
    OldMissing,
    NewMissing,
    BothMissing,
}

/// A file that was modified within a [`Diff`].
#[cfg_attr(feature = "serde", derive(Serialize), serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Modified {
    /// The path to this file, relative to the repository root.
    pub path: PathBuf,
    /// The set of [`Modification`]s to this file.
    pub diff: FileDiff<Modification>,
    /// Was there an EOF newline present.
    pub eof: Option<EofNewLine>,
}

/// The set of changes for a given file.
#[cfg_attr(
    feature = "serde",
    derive(Serialize),
    serde(tag = "type", rename_all = "camelCase")
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileDiff<T> {
    /// The file is a binary file and so no set of changes can be provided.
    Binary,
    /// The set of changes, as [`Hunks`] for a plaintext file.
    #[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
    Plain { hunks: Hunks<T> },
}

/// Statistics describing a particular [`Diff`].
#[cfg_attr(feature = "serde", derive(Serialize), serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Stats {
    /// Get the total number of files changed in a [`Diff`]
    pub files_changed: usize,
    /// Get the total number of insertions in a [`Diff`].
    pub insertions: usize,
    /// Get the total number of deletions in a [`Diff`].
    pub deletions: usize,
}

/// A set of changes across multiple lines.
///
/// The parameter `T` can be an [`Addition`], [`Deletion`], or
/// [`Modification`].
#[cfg_attr(feature = "serde", derive(Serialize), serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hunk<T> {
    pub header: Line,
    pub lines: Vec<T>,
}

/// A set of [`Hunk`] changes.
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hunks<T>(pub Vec<Hunk<T>>);

impl<T> Default for Hunks<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T> Hunks<T> {
    pub fn iter(&self) -> impl Iterator<Item = &Hunk<T>> {
        self.0.iter()
    }
}

impl<T> From<Vec<Hunk<T>>> for Hunks<T> {
    fn from(hunks: Vec<Hunk<T>>) -> Self {
        Self(hunks)
    }
}

/// The content of a single line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line(pub(crate) Vec<u8>);

impl From<Vec<u8>> for Line {
    fn from(v: Vec<u8>) -> Self {
        Self(v)
    }
}

impl From<String> for Line {
    fn from(s: String) -> Self {
        Self(s.into_bytes())
    }
}

#[cfg(feature = "serde")]
impl Serialize for Line {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = std::str::from_utf8(&self.0).map_err(ser::Error::custom)?;

        serializer.serialize_str(s)
    }
}

/// Either the modification of a single [`Line`], or just contextual
/// information.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Modification {
    /// A lines is an addition in a file.
    Addition(Addition),

    /// A lines is a deletion in a file.
    Deletion(Deletion),

    /// A contextual line in a file, i.e. there were no changes to the line.
    Context {
        line: Line,
        line_no_old: u32,
        line_no_new: u32,
    },
}

#[cfg(feature = "serde")]
impl Serialize for Modification {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap as _;

        const NAME: &str = "Modification";
        const ADDITION: &str = "Addition";
        const DELETION: &str = "Deletion";
        const CONTEXT: &str = "Context";

        match self {
            Modification::Addition(addition) => {
                let mut map = serializer.serialize_map(Some(3))?;
                map.serialize_entry("line", &addition.line)?;
                map.serialize_entry("lineNo", &addition.line_no)?;
                map.serialize_entry("type", "addition")?;
                map.end()
            },
            Modification::Deletion(deletion) => {
                let mut map = serializer.serialize_map(Some(3))?;
                map.serialize_entry("line", &deletion.line)?;
                map.serialize_entry("lineNo", &deletion.line_no)?;
                map.serialize_entry("type", "deletion")?;
                map.end()
            },
            Modification::Context {
                line,
                line_no_old,
                line_no_new,
            } => {
                let mut map = serializer.serialize_map(Some(4))?;
                map.serialize_entry("line", line)?;
                map.serialize_entry("lineNoOld", line_no_old)?;
                map.serialize_entry("lineNoNew", line_no_new)?;
                map.serialize_entry("type", "context")?;
                map.end()
            },
        }
    }
}

/// A addition of a [`Line`] at the `line_no`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Addition {
    pub line: Line,
    pub line_no: u32,
}

#[cfg(feature = "serde")]
impl Serialize for Addition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct as _;

        let mut s = serializer.serialize_struct("Addition", 3)?;
        s.serialize_field("line", &self.line)?;
        s.serialize_field("lineNo", &self.line_no)?;
        s.serialize_field("type", "addition")?;
        s.end()
    }
}

/// A deletion of a [`Line`] at the `line_no`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Deletion {
    pub line: Line,
    pub line_no: u32,
}

#[cfg(feature = "serde")]
impl Serialize for Deletion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct as _;

        let mut s = serializer.serialize_struct("Deletion", 3)?;
        s.serialize_field("line", &self.line)?;
        s.serialize_field("lineNo", &self.line_no)?;
        s.serialize_field("type", "deletion")?;
        s.end()
    }
}

impl Modification {
    pub fn addition(line: impl Into<Line>, line_no: u32) -> Self {
        Self::Addition(Addition {
            line: line.into(),
            line_no,
        })
    }

    pub fn deletion(line: impl Into<Line>, line_no: u32) -> Self {
        Self::Deletion(Deletion {
            line: line.into(),
            line_no,
        })
    }

    pub fn context(line: impl Into<Line>, line_no_old: u32, line_no_new: u32) -> Self {
        Self::Context {
            line: line.into(),
            line_no_old,
            line_no_new,
        }
    }
}
