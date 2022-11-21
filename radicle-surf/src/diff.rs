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

#![allow(dead_code, unused_variables, missing_docs)]

use std::{convert::TryFrom, slice};

#[cfg(feature = "serialize")]
use serde::{ser, Serialize, Serializer};

use crate::{
    file_system::{Directory, Path},
    git::{Error, Repository},
};

pub mod git;

#[cfg_attr(
    feature = "serialize",
    derive(Serialize),
    serde(rename_all = "camelCase")
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diff {
    pub created: Vec<CreateFile>,
    pub deleted: Vec<DeleteFile>,
    pub moved: Vec<MoveFile>,
    pub copied: Vec<CopyFile>,
    pub modified: Vec<ModifiedFile>,
}

impl Default for Diff {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(feature = "serialize", derive(Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateFile {
    pub path: Path,
    pub diff: FileDiff,
}

#[cfg_attr(feature = "serialize", derive(Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeleteFile {
    pub path: Path,
    pub diff: FileDiff,
}

#[cfg_attr(
    feature = "serialize",
    derive(Serialize),
    serde(rename_all = "camelCase")
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoveFile {
    pub old_path: Path,
    pub new_path: Path,
}

#[cfg_attr(
    feature = "serialize",
    derive(Serialize),
    serde(rename_all = "camelCase")
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CopyFile {
    pub old_path: Path,
    pub new_path: Path,
}

#[cfg_attr(
    feature = "serialize",
    derive(Serialize),
    serde(rename_all = "camelCase")
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EofNewLine {
    OldMissing,
    NewMissing,
    BothMissing,
}

#[cfg_attr(
    feature = "serialize",
    derive(Serialize),
    serde(rename_all = "camelCase")
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModifiedFile {
    pub path: Path,
    pub diff: FileDiff,
    pub eof: Option<EofNewLine>,
}

/// A set of changes belonging to one file.
#[cfg_attr(
    feature = "serialize",
    derive(Serialize),
    serde(tag = "type", rename_all = "camelCase")
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileDiff {
    Binary,
    #[cfg_attr(feature = "serialize", serde(rename_all = "camelCase"))]
    Plain {
        hunks: Hunks,
    },
}

/// Statistics describing a particular [`Diff`].
#[cfg_attr(
    feature = "serialize",
    derive(Serialize),
    serde(rename_all = "camelCase")
)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stats {
    /// Get the total number of files changed in a diff.
    pub files_changed: usize,
    /// Get the total number of insertions in a diff.
    pub insertions: usize,
    /// Get the total number of deletions in a diff.
    pub deletions: usize,
}

/// A set of line changes.
#[cfg_attr(
    feature = "serialize",
    derive(Serialize),
    serde(rename_all = "camelCase")
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hunk {
    pub header: Line,
    pub lines: Vec<LineDiff>,
}

/// A set of [`Hunk`]s.
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hunks(pub Vec<Hunk>);

pub struct IterHunks<'a> {
    inner: slice::Iter<'a, Hunk>,
}

impl Hunks {
    pub fn iter(&self) -> IterHunks<'_> {
        IterHunks {
            inner: self.0.iter(),
        }
    }
}

impl From<Vec<Hunk>> for Hunks {
    fn from(hunks: Vec<Hunk>) -> Self {
        Self(hunks)
    }
}

impl<'a> Iterator for IterHunks<'a> {
    type Item = &'a Hunk;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl TryFrom<git2::Patch<'_>> for Hunks {
    type Error = git::error::Hunk;

    fn try_from(patch: git2::Patch) -> Result<Self, Self::Error> {
        let mut hunks = Vec::new();
        for h in 0..patch.num_hunks() {
            let (hunk, hunk_lines) = patch.hunk(h)?;
            let header = Line(hunk.header().to_owned());
            let mut lines: Vec<LineDiff> = Vec::new();

            for l in 0..hunk_lines {
                let line = patch.line_in_hunk(h, l)?;
                let line = LineDiff::try_from(line)?;
                lines.push(line);
            }
            hunks.push(Hunk { header, lines });
        }
        Ok(Hunks(hunks))
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

#[cfg(feature = "serialize")]
impl Serialize for Line {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = std::str::from_utf8(&self.0).map_err(ser::Error::custom)?;

        serializer.serialize_str(s)
    }
}

/// Single line delta. Two of these are need to represented a modified line: one
/// addition and one deletion. Context is also represented with this type.
#[cfg_attr(
    feature = "serialize",
    derive(Serialize),
    serde(tag = "type", rename_all = "camelCase")
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineDiff {
    /// Line added.
    #[cfg_attr(feature = "serialize", serde(rename_all = "camelCase"))]
    Addition { line: Line, line_num: u32 },

    /// Line deleted.
    #[cfg_attr(feature = "serialize", serde(rename_all = "camelCase"))]
    Deletion { line: Line, line_num: u32 },

    /// Line context.
    #[cfg_attr(feature = "serialize", serde(rename_all = "camelCase"))]
    Context {
        line: Line,
        line_num_old: u32,
        line_num_new: u32,
    },
}

impl LineDiff {
    pub fn addition(line: impl Into<Line>, line_num: u32) -> Self {
        Self::Addition {
            line: line.into(),
            line_num,
        }
    }

    pub fn deletion(line: impl Into<Line>, line_num: u32) -> Self {
        Self::Deletion {
            line: line.into(),
            line_num,
        }
    }

    pub fn context(line: impl Into<Line>, line_num_old: u32, line_num_new: u32) -> Self {
        Self::Context {
            line: line.into(),
            line_num_old,
            line_num_new,
        }
    }
}

impl Diff {
    pub fn new() -> Self {
        Diff {
            created: Vec::new(),
            deleted: Vec::new(),
            moved: Vec::new(),
            copied: Vec::new(),
            modified: Vec::new(),
        }
    }

    // TODO: Direction of comparison is not obvious with this signature.
    // For now using conventional approach with the right being "newer".
    #[allow(clippy::self_named_constructors)]
    pub fn diff(left: Directory, right: Directory, repo: Repository) -> Result<Self, Error> {
        // TODO: Some of the deleted files may actually be moved (renamed) to one of the
        // created files. Finding out which of the deleted files were deleted
        // and which were moved will probably require performing some variant of
        // the longest common substring algorithm for each pair in D x C. Final
        // decision can be based on heuristics, e.g. the file can be considered
        // moved, if len(LCS) > 0,25 * min(size(d), size(c)), and
        // deleted otherwise.
        repo.diff(left, right)
    }

    pub(crate) fn add_modified_file(
        &mut self,
        path: Path,
        hunks: impl Into<Hunks>,
        eof: Option<EofNewLine>,
    ) {
        // TODO: file diff can be calculated at this point
        // Use pijul's transaction diff as an inspiration?
        // https://nest.pijul.com/pijul_org/pijul:master/1468b7281a6f3785e9#anesp4Qdq3V
        self.modified.push(ModifiedFile {
            path,
            diff: FileDiff::Plain {
                hunks: hunks.into(),
            },
            eof,
        });
    }

    pub(crate) fn add_moved_file(&mut self, old_path: Path, new_path: Path) {
        self.moved.push(MoveFile { old_path, new_path });
    }

    pub(crate) fn add_copied_file(&mut self, old_path: Path, new_path: Path) {
        self.copied.push(CopyFile { old_path, new_path });
    }

    pub(crate) fn add_modified_binary_file(&mut self, path: Path) {
        self.modified.push(ModifiedFile {
            path,
            diff: FileDiff::Binary,
            eof: None,
        });
    }

    pub(crate) fn add_created_file(&mut self, path: Path, diff: FileDiff) {
        self.created.push(CreateFile { path, diff });
    }

    pub(crate) fn add_deleted_file(&mut self, path: Path, diff: FileDiff) {
        self.deleted.push(DeleteFile { path, diff });
    }

    pub fn stats(&self) -> Stats {
        let mut deletions = 0;
        let mut insertions = 0;

        for file in &self.modified {
            if let self::FileDiff::Plain { ref hunks } = file.diff {
                for hunk in hunks.iter() {
                    for line in &hunk.lines {
                        match line {
                            self::LineDiff::Addition { .. } => insertions += 1,
                            self::LineDiff::Deletion { .. } => deletions += 1,
                            _ => {},
                        }
                    }
                }
            }
        }

        for file in &self.created {
            if let self::FileDiff::Plain { ref hunks } = file.diff {
                for hunk in hunks.iter() {
                    for line in &hunk.lines {
                        if let self::LineDiff::Addition { .. } = line {
                            insertions += 1
                        }
                    }
                }
            }
        }

        for file in &self.deleted {
            if let self::FileDiff::Plain { ref hunks } = file.diff {
                for hunk in hunks.iter() {
                    for line in &hunk.lines {
                        if let self::LineDiff::Deletion { .. } = line {
                            deletions += 1
                        }
                    }
                }
            }
        }

        Stats {
            files_changed: self.modified.len() + self.created.len() + self.deleted.len(),
            insertions,
            deletions,
        }
    }
}
