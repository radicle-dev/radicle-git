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

use std::{
    borrow::Cow,
    ops::Range,
    path::{Path, PathBuf},
    string::FromUtf8Error,
};

#[cfg(feature = "serde")]
use serde::{ser, ser::SerializeStruct, Serialize, Serializer};

use git_ext::Oid;

pub mod git;

/// The serializable representation of a `git diff`.
///
/// A [`Diff`] can be retrieved by the following functions:
///    * [`crate::Repository::diff`]
///    * [`crate::Repository::diff_commit`]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Diff {
    files: Vec<FileDiff>,
    stats: Stats,
}

impl Diff {
    /// Creates an empty diff.
    pub(crate) fn new() -> Self {
        Diff::default()
    }

    /// Returns an iterator of the file in the diff.
    pub fn files(&self) -> impl Iterator<Item = &FileDiff> {
        self.files.iter()
    }

    /// Returns owned files in the diff.
    pub fn into_files(self) -> Vec<FileDiff> {
        self.files
    }

    pub fn added(&self) -> impl Iterator<Item = &Added> {
        self.files().filter_map(|x| match x {
            FileDiff::Added(a) => Some(a),
            _ => None,
        })
    }

    pub fn deleted(&self) -> impl Iterator<Item = &Deleted> {
        self.files().filter_map(|x| match x {
            FileDiff::Deleted(a) => Some(a),
            _ => None,
        })
    }

    pub fn moved(&self) -> impl Iterator<Item = &Moved> {
        self.files().filter_map(|x| match x {
            FileDiff::Moved(a) => Some(a),
            _ => None,
        })
    }

    pub fn modified(&self) -> impl Iterator<Item = &Modified> {
        self.files().filter_map(|x| match x {
            FileDiff::Modified(a) => Some(a),
            _ => None,
        })
    }

    pub fn copied(&self) -> impl Iterator<Item = &Copied> {
        self.files().filter_map(|x| match x {
            FileDiff::Copied(a) => Some(a),
            _ => None,
        })
    }

    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    fn insert_modified(&mut self, path: PathBuf, diff: DiffContent, old: DiffFile, new: DiffFile) {
        let diff = FileDiff::Modified(Modified {
            path,
            diff,
            old,
            new,
        });
        self.files.push(diff)
    }

    fn insert_moved(&mut self, old_path: PathBuf, new_path: PathBuf) {
        let diff = FileDiff::Moved(Moved {
            old_path,
            new_path,
            diff: DiffContent::Empty,
        });
        self.files.push(diff);
    }

    fn insert_copied(&mut self, old_path: PathBuf, new_path: PathBuf) {
        let diff = FileDiff::Copied(Copied {
            old_path,
            new_path,
            diff: DiffContent::Empty,
        });
        self.files.push(diff);
    }

    fn insert_added(&mut self, path: PathBuf, diff: DiffContent, new: DiffFile) {
        let diff = FileDiff::Added(Added { path, diff, new });
        self.files.push(diff);
    }

    fn insert_deleted(&mut self, path: PathBuf, diff: DiffContent, old: DiffFile) {
        let diff = FileDiff::Deleted(Deleted { path, diff, old });
        self.files.push(diff);
    }
}

/// A file that was added within a [`Diff`].
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Added {
    /// The path to this file, relative to the repository root.
    pub path: PathBuf,
    pub diff: DiffContent,
    pub new: DiffFile,
}

/// A file that was deleted within a [`Diff`].
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Deleted {
    /// The path to this file, relative to the repository root.
    pub path: PathBuf,
    pub diff: DiffContent,
    pub old: DiffFile,
}

/// A file that was moved within a [`Diff`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Moved {
    /// The old path to this file, relative to the repository root.
    pub old_path: PathBuf,
    /// The new path to this file, relative to the repository root.
    pub new_path: PathBuf,
    pub diff: DiffContent,
}

#[cfg(feature = "serde")]
impl Serialize for Moved {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Moved", 2)?;
        state.serialize_field("oldPath", &self.old_path)?;
        state.serialize_field("newPath", &self.new_path)?;
        // `DiffContent` is not serialized yet for `Moved`, only
        // to keep the serialization same as before.
        state.end()
    }
}

/// A file that was copied within a [`Diff`].
#[cfg_attr(feature = "serde", derive(Serialize), serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Copied {
    /// The old path to this file, relative to the repository root.
    pub old_path: PathBuf,
    /// The new path to this file, relative to the repository root.
    pub new_path: PathBuf,
    pub diff: DiffContent,
}

#[cfg_attr(feature = "serde", derive(Serialize), serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EofNewLine {
    OldMissing,
    NewMissing,
    BothMissing,
    NoneMissing,
}

impl Default for EofNewLine {
    fn default() -> Self {
        Self::NoneMissing
    }
}

/// A file that was modified within a [`Diff`].
#[cfg_attr(feature = "serde", derive(Serialize), serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Modified {
    pub path: PathBuf,
    pub diff: DiffContent,
    pub old: DiffFile,
    pub new: DiffFile,
}

/// The set of changes for a given file.
#[cfg_attr(
    feature = "serde",
    derive(Serialize),
    serde(tag = "type", rename_all = "camelCase")
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffContent {
    /// The file is a binary file and so no set of changes can be provided.
    Binary,
    /// The set of changes, as [`Hunks`] for a plaintext file.
    #[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
    Plain {
        hunks: Hunks<Modification>,
        eof: EofNewLine,
    },
    /// The set of changes, as [`Hunks`] for a diff file.
    #[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
    Diff {
        hunks: Hunks<DiffModification>,
        eof: EofNewLine,
    },
    Empty,
}

impl DiffContent {
    pub fn eof(&self) -> Option<EofNewLine> {
        match self {
            Self::Plain { hunks: _, eof } => Some(eof.clone()),
            _ => None,
        }
    }
}

/// File mode in a diff.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize), serde(rename_all = "camelCase"))]
pub enum FileMode {
    /// For regular files.
    Blob,
    /// For regular files that are executable.
    BlobExecutable,
    /// For directories.
    Tree,
    /// For symbolic links.
    Link,
    /// Used for Git submodules.
    Commit,
}

impl From<FileMode> for u32 {
    fn from(m: FileMode) -> Self {
        git2::FileMode::from(m).into()
    }
}

impl From<FileMode> for i32 {
    fn from(m: FileMode) -> Self {
        git2::FileMode::from(m).into()
    }
}

/// A modified file.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize), serde(rename_all = "camelCase"))]
pub struct DiffFile {
    /// File blob id.
    pub oid: Oid,
    /// File mode.
    pub mode: FileMode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileDiff {
    Added(Added),
    Deleted(Deleted),
    Modified(Modified),
    Moved(Moved),
    Copied(Copied),
}

impl FileDiff {
    pub fn path(&self) -> &Path {
        match self {
            FileDiff::Added(x) => x.path.as_path(),
            FileDiff::Deleted(x) => x.path.as_path(),
            FileDiff::Modified(x) => x.path.as_path(),
            FileDiff::Moved(x) => x.new_path.as_path(),
            FileDiff::Copied(x) => x.new_path.as_path(),
        }
    }
}

#[cfg(feature = "serde")]
impl Serialize for FileDiff {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("FileDiff", 7)?;
        match &self {
            FileDiff::Added(x) => {
                state.serialize_field("path", &x.path)?;
                state.serialize_field("diff", &x.diff)?
            },
            FileDiff::Deleted(x) => {
                state.serialize_field("path", &x.path)?;
                state.serialize_field("diff", &x.diff)?
            },
            FileDiff::Modified(x) => {
                state.serialize_field("path", &x.path)?;
                state.serialize_field("diff", &x.diff)?;
            },
            FileDiff::Moved(x) => {
                state.serialize_field("oldPath", &x.old_path)?;
                state.serialize_field("newPath", &x.new_path)?
            },
            FileDiff::Copied(x) => {
                state.serialize_field("oldPath", &x.old_path)?;
                state.serialize_field("newPath", &x.new_path)?
            },
        }
        state.end()
    }
}

#[cfg(feature = "serde")]
impl Serialize for Diff {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Diff", 6)?;
        state.serialize_field("added", &self.added().collect::<Vec<_>>())?;
        state.serialize_field("deleted", &self.deleted().collect::<Vec<_>>())?;
        state.serialize_field("moved", &self.moved().collect::<Vec<_>>())?;
        state.serialize_field("copied", &self.copied().collect::<Vec<_>>())?;
        state.serialize_field("modified", &self.modified().collect::<Vec<_>>())?;
        state.serialize_field("stats", &self.stats())?;
        state.end()
    }
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
    /// Old line range.
    pub old: Range<u32>,
    /// New line range.
    pub new: Range<u32>,
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

impl Line {
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }

    pub fn from_utf8(self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.0)
    }

    pub fn from_utf8_lossy(&self) -> Cow<str> {
        String::from_utf8_lossy(&self.0)
    }
}

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

/// Either the modification of a single diff [`Line`], or just contextual
/// information.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffModification {
    /// An addition line is to be added.
    AdditionAddition(AdditionAddition),
    AdditionContext {
        line: Line,
        line_no_old: u32,
        line_no_new: u32,
    },
    /// An addition line is to be removed.
    AdditionDeletion(AdditionDeletion),
    /// A context line is to be added.
    ContextAddition(ContextAddition),
    /// A contextual line in a file, i.e. there were no changes to the line.
    ContextContext {
        line: Line,
        line_no_old: u32,
        line_no_new: u32,
    },
    /// A context line is to be removed.
    ContextDeletion(ContextDeletion),
    /// A deletion line is to be added.
    DeletionAddition(DeletionAddition),
    /// A deletion line in a diff, i.e. there were no changes to the line.
    DeletionContext {
        line: Line,
        line_no_old: u32,
        line_no_new: u32,
    },
    /// A deletion line is to be removed.
    DeletionDeletion(DeletionDeletion),
}

#[cfg(feature = "serde")]
impl Serialize for DiffModification {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap as _;

        match self {
            Self::AdditionAddition(v) => {
                let mut map = serializer.serialize_map(Some(3))?;
                map.serialize_entry("line", &v.line)?;
                map.serialize_entry("lineNo", &v.line_no)?;
                map.serialize_entry("type", "addition_addition")?;
                map.end()
            },
            Self::AdditionContext {
                line,
                line_no_old,
                line_no_new,
            } => {
                let mut map = serializer.serialize_map(Some(4))?;
                map.serialize_entry("line", line)?;
                map.serialize_entry("lineNoOld", line_no_old)?;
                map.serialize_entry("lineNoNew", line_no_new)?;
                map.serialize_entry("type", "addition_context")?;
                map.end()
            },
            Self::AdditionDeletion(v) => {
                let mut map = serializer.serialize_map(Some(3))?;
                map.serialize_entry("line", &v.line)?;
                map.serialize_entry("lineNo", &v.line_no)?;
                map.serialize_entry("type", "addition_deletion")?;
                map.end()
            },
            Self::ContextAddition(v) => {
                let mut map = serializer.serialize_map(Some(3))?;
                map.serialize_entry("line", &v.line)?;
                map.serialize_entry("lineNo", &v.line_no)?;
                map.serialize_entry("type", "context_addition")?;
                map.end()
            },
            Self::ContextContext {
                line,
                line_no_old,
                line_no_new,
            } => {
                let mut map = serializer.serialize_map(Some(4))?;
                map.serialize_entry("line", line)?;
                map.serialize_entry("lineNoOld", line_no_old)?;
                map.serialize_entry("lineNoNew", line_no_new)?;
                map.serialize_entry("type", "context_context")?;
                map.end()
            },
            Self::ContextDeletion(v) => {
                let mut map = serializer.serialize_map(Some(3))?;
                map.serialize_entry("line", &v.line)?;
                map.serialize_entry("lineNo", &v.line_no)?;
                map.serialize_entry("type", "context_deletion")?;
                map.end()
            },
            Self::DeletionAddition(v) => {
                let mut map = serializer.serialize_map(Some(3))?;
                map.serialize_entry("line", &v.line)?;
                map.serialize_entry("lineNo", &v.line_no)?;
                map.serialize_entry("type", "deletion_addition")?;
                map.end()
            },
            Self::DeletionContext {
                line,
                line_no_old,
                line_no_new,
            } => {
                let mut map = serializer.serialize_map(Some(4))?;
                map.serialize_entry("line", line)?;
                map.serialize_entry("lineNoOld", line_no_old)?;
                map.serialize_entry("lineNoNew", line_no_new)?;
                map.serialize_entry("type", "deletion_context")?;
                map.end()
            },
            Self::DeletionDeletion(v) => {
                let mut map = serializer.serialize_map(Some(3))?;
                map.serialize_entry("line", &v.line)?;
                map.serialize_entry("lineNo", &v.line_no)?;
                map.serialize_entry("type", "deletion_deletion")?;
                map.end()
            },
        }
    }
}

impl DiffModification {
    pub fn addition_addition(line: Line, line_no: u32) -> Self {
        Self::AdditionAddition(AdditionAddition { line, line_no })
    }

    pub fn addition_deletion(line: Line, line_no: u32) -> Self {
        Self::AdditionDeletion(AdditionDeletion { line, line_no })
    }

    pub fn context_addition(line: Line, line_no: u32) -> Self {
        Self::ContextAddition(ContextAddition { line, line_no })
    }

    pub fn deletion_addition(line: Line, line_no: u32) -> Self {
        Self::DeletionAddition(DeletionAddition { line, line_no })
    }

    pub fn deletion_deletion(line: Line, line_no: u32) -> Self {
        Self::DeletionDeletion(DeletionDeletion { line, line_no })
    }

    pub fn context_deletion(line: Line, line_no: u32) -> Self {
        Self::ContextDeletion(ContextDeletion { line, line_no })
    }
}

/// An addition of a [`Line`] addition at the `line_no`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdditionAddition {
    pub line: Line,
    pub line_no: u32,
}

#[cfg(feature = "serde")]
impl Serialize for AdditionAddition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct as _;

        let mut s = serializer.serialize_struct("Addition", 3)?;
        s.serialize_field("line", &self.line)?;
        s.serialize_field("lineNo", &self.line_no)?;
        s.serialize_field("type", "addition_addition")?;
        s.end()
    }
}

/// A deletion of a [`Line`] addition at the `line_no`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdditionDeletion {
    pub line: Line,
    pub line_no: u32,
}

#[cfg(feature = "serde")]
impl Serialize for AdditionDeletion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct as _;

        let mut s = serializer.serialize_struct("AdditionDeletion", 3)?;
        s.serialize_field("line", &self.line)?;
        s.serialize_field("lineNo", &self.line_no)?;
        s.serialize_field("type", "addition_deletion")?;
        s.end()
    }
}

/// An addition of a [`Line`] context at the `line_no`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextAddition {
    pub line: Line,
    pub line_no: u32,
}

#[cfg(feature = "serde")]
impl Serialize for ContextAddition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct as _;

        let mut s = serializer.serialize_struct("ContextAddition", 3)?;
        s.serialize_field("line", &self.line)?;
        s.serialize_field("lineNo", &self.line_no)?;
        s.serialize_field("type", "context_addition")?;
        s.end()
    }
}

/// An addition of a [`Line`] deletion at the `line_no`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeletionAddition {
    pub line: Line,
    pub line_no: u32,
}

#[cfg(feature = "serde")]
impl Serialize for DeletionAddition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct as _;

        let mut s = serializer.serialize_struct("DeletionAddition", 3)?;
        s.serialize_field("line", &self.line)?;
        s.serialize_field("lineNo", &self.line_no)?;
        s.serialize_field("type", "deletion_addition")?;
        s.end()
    }
}

/// A deletion of a [`Line`] deletion at the `line_no`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeletionDeletion {
    pub line: Line,
    pub line_no: u32,
}
#[cfg(feature = "serde")]
impl Serialize for DeletionDeletion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct as _;

        let mut s = serializer.serialize_struct("DeletionDeletion", 3)?;
        s.serialize_field("line", &self.line)?;
        s.serialize_field("lineNo", &self.line_no)?;
        s.serialize_field("type", "deletion_deletion")?;
        s.end()
    }
}

/// A deletion of a [`Line`] of context at the `line_no`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextDeletion {
    pub line: Line,
    pub line_no: u32,
}
#[cfg(feature = "serde")]
impl Serialize for ContextDeletion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct as _;

        let mut s = serializer.serialize_struct("ContextDeletion", 3)?;
        s.serialize_field("line", &self.line)?;
        s.serialize_field("lineNo", &self.line_no)?;
        s.serialize_field("type", "context_deletion")?;
        s.end()
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
