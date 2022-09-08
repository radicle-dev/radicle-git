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

use std::{cell::RefCell, cmp::Ordering, convert::TryFrom, ops::Deref, rc::Rc, slice};

#[cfg(feature = "serialize")]
use serde::{ser, Serialize, Serializer};

use crate::file_system::{Directory, DirectoryContents, Path};

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
    pub fn diff(left: Directory, right: Directory) -> Self {
        let mut diff = Diff::new();
        let path = Rc::new(RefCell::new(Path::from_labels(right.current(), &[])));
        Diff::collect_diff(&left, &right, &path, &mut diff);

        // TODO: Some of the deleted files may actually be moved (renamed) to one of the
        // created files. Finding out which of the deleted files were deleted
        // and which were moved will probably require performing some variant of
        // the longest common substring algorithm for each pair in D x C. Final
        // decision can be based on heuristics, e.g. the file can be considered
        // moved, if len(LCS) > 0,25 * min(size(d), size(c)), and
        // deleted otherwise.

        diff
    }

    fn collect_diff(
        old: &Directory,
        new: &Directory,
        parent_path: &Rc<RefCell<Path>>,
        diff: &mut Diff,
    ) {
        let mut old_iter = old.iter();
        let mut new_iter = new.iter();
        let mut old_entry_opt = old_iter.next();
        let mut new_entry_opt = new_iter.next();

        while old_entry_opt.is_some() || new_entry_opt.is_some() {
            match (&old_entry_opt, &new_entry_opt) {
                (Some(ref old_entry), Some(ref new_entry)) => {
                    match new_entry.label().cmp(&old_entry.label()) {
                        Ordering::Greater => {
                            diff.add_deleted_files(old_entry, parent_path);
                            old_entry_opt = old_iter.next();
                        },
                        Ordering::Less => {
                            diff.add_created_files(new_entry, parent_path);
                            new_entry_opt = new_iter.next();
                        },
                        Ordering::Equal => match (new_entry, old_entry) {
                            (
                                DirectoryContents::File {
                                    name: new_file_name,
                                    file: new_file,
                                },
                                DirectoryContents::File {
                                    name: old_file_name,
                                    file: old_file,
                                },
                            ) => {
                                if old_file.size != new_file.size
                                    || old_file.checksum() != new_file.checksum()
                                {
                                    let mut path = parent_path.borrow().clone();
                                    path.push(new_file_name.clone());

                                    diff.add_modified_file(path, vec![], None);
                                }
                                old_entry_opt = old_iter.next();
                                new_entry_opt = new_iter.next();
                            },
                            (
                                DirectoryContents::File {
                                    name: new_file_name,
                                    file: new_file,
                                },
                                DirectoryContents::Directory(old_dir),
                            ) => {
                                let mut path = parent_path.borrow().clone();
                                path.push(new_file_name.clone());

                                diff.add_created_file(
                                    path,
                                    FileDiff::Plain {
                                        hunks: Hunks::default(),
                                    },
                                );
                                diff.add_deleted_files(old_entry, parent_path);

                                old_entry_opt = old_iter.next();
                                new_entry_opt = new_iter.next();
                            },
                            (
                                DirectoryContents::Directory(new_dir),
                                DirectoryContents::File {
                                    name: old_file_name,
                                    file: old_file,
                                },
                            ) => {
                                let mut path = parent_path.borrow().clone();
                                path.push(old_file_name.clone());

                                diff.add_created_files(new_entry, parent_path);
                                diff.add_deleted_file(
                                    path,
                                    FileDiff::Plain {
                                        hunks: Hunks::default(),
                                    },
                                );

                                old_entry_opt = old_iter.next();
                                new_entry_opt = new_iter.next();
                            },
                            (
                                DirectoryContents::Directory(new_dir),
                                DirectoryContents::Directory(old_dir),
                            ) => {
                                parent_path.borrow_mut().push(new_dir.current().clone());
                                Diff::collect_diff(
                                    old_dir.deref(),
                                    new_dir.deref(),
                                    parent_path,
                                    diff,
                                );
                                parent_path.borrow_mut().pop();
                                old_entry_opt = old_iter.next();
                                new_entry_opt = new_iter.next();
                            },
                        },
                    }
                },
                (Some(ref old_entry), None) => {
                    diff.add_deleted_files(old_entry, parent_path);
                    old_entry_opt = old_iter.next();
                },
                (None, Some(ref new_entry)) => {
                    diff.add_created_files(new_entry, parent_path);
                    new_entry_opt = new_iter.next();
                },
                (None, None) => break,
            }
        }
    }

    // if entry is a file, then return this file,
    // or a list of files in the directory tree otherwise
    fn collect_files_from_entry<F, T>(
        entry: &DirectoryContents,
        parent_path: &Rc<RefCell<Path>>,
        mapper: F,
    ) -> Vec<T>
    where
        F: Fn(Path) -> T + Copy,
    {
        match entry {
            DirectoryContents::Directory(dir) => Diff::collect_files(dir, parent_path, mapper),
            DirectoryContents::File { name, .. } => {
                let mut path = parent_path.borrow().clone();
                path.push(name.clone());

                vec![mapper(path)]
            },
        }
    }

    fn collect_files<F, T>(dir: &Directory, parent_path: &Rc<RefCell<Path>>, mapper: F) -> Vec<T>
    where
        F: Fn(Path) -> T + Copy,
    {
        let mut files: Vec<T> = Vec::new();
        Diff::collect_files_inner(dir, parent_path, mapper, &mut files);
        files
    }

    fn collect_files_inner<'a, F, T>(
        dir: &'a Directory,
        parent_path: &Rc<RefCell<Path>>,
        mapper: F,
        files: &mut Vec<T>,
    ) where
        F: Fn(Path) -> T + Copy,
    {
        parent_path.borrow_mut().push(dir.current());
        for entry in dir.iter() {
            match entry {
                DirectoryContents::Directory(subdir) => {
                    Diff::collect_files_inner(&subdir, parent_path, mapper, files);
                },
                DirectoryContents::File { name, .. } => {
                    let mut path = parent_path.borrow().clone();
                    path.push(name);
                    files.push(mapper(path));
                },
            }
        }
        parent_path.borrow_mut().pop();
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

    fn add_created_files(&mut self, dc: &DirectoryContents, parent_path: &Rc<RefCell<Path>>) {
        let mut new_files: Vec<CreateFile> =
            Diff::collect_files_from_entry(dc, parent_path, |path| CreateFile {
                path,
                diff: FileDiff::Plain {
                    hunks: Hunks::default(),
                },
            });
        self.created.append(&mut new_files);
    }

    pub(crate) fn add_deleted_file(&mut self, path: Path, diff: FileDiff) {
        self.deleted.push(DeleteFile { path, diff });
    }

    fn add_deleted_files(&mut self, dc: &DirectoryContents, parent_path: &Rc<RefCell<Path>>) {
        let mut new_files: Vec<DeleteFile> =
            Diff::collect_files_from_entry(dc, parent_path, |path| DeleteFile {
                path,
                diff: FileDiff::Plain {
                    hunks: Hunks::default(),
                },
            });
        self.deleted.append(&mut new_files);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        diff::*,
        file_system::{unsound, *},
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn test_create_file() {
        let directory = Directory::root();

        let mut new_directory = Directory::root();
        new_directory.insert_file(unsound::path::new("banana.rs"), File::new(b"use banana"));

        let diff = Diff::diff(directory, new_directory);

        let expected_diff = Diff {
            created: vec![CreateFile {
                path: Path::with_root(&[unsound::label::new("banana.rs")]),
                diff: FileDiff::Plain {
                    hunks: Hunks::default(),
                },
            }],
            deleted: vec![],
            copied: vec![],
            moved: vec![],
            modified: vec![],
        };

        assert_eq!(diff, expected_diff)
    }

    #[test]
    fn test_delete_file() {
        let mut directory = Directory::root();
        directory.insert_file(unsound::path::new("banana.rs"), File::new(b"use banana"));

        let new_directory = Directory::root();

        let diff = Diff::diff(directory, new_directory);

        let expected_diff = Diff {
            created: vec![],
            deleted: vec![DeleteFile {
                path: Path::with_root(&[unsound::label::new("banana.rs")]),
                diff: FileDiff::Plain {
                    hunks: Hunks::default(),
                },
            }],
            moved: vec![],
            copied: vec![],
            modified: vec![],
        };

        assert_eq!(diff, expected_diff)
    }

    /* TODO(fintan): Move is not detected yet
    #[test]
    fn test_moved_file() {
        let mut directory = Directory::root();
        directory.insert_file(&unsound::path::new("mod.rs"), File::new(b"use banana"));

        let mut new_directory = Directory::root();
        new_directory.insert_file(&unsound::path::new("banana.rs"), File::new(b"use banana"));

        let diff = Diff::diff(directory, new_directory).expect("diff failed");

        assert_eq!(diff, Diff::new())
    }
    */

    #[test]
    fn test_modify_file() {
        let mut directory = Directory::root();
        directory.insert_file(unsound::path::new("banana.rs"), File::new(b"use banana"));

        let mut new_directory = Directory::root();
        new_directory.insert_file(unsound::path::new("banana.rs"), File::new(b"use banana;"));

        let diff = Diff::diff(directory, new_directory);

        let expected_diff = Diff {
            created: vec![],
            deleted: vec![],
            moved: vec![],
            copied: vec![],
            modified: vec![ModifiedFile {
                path: Path::with_root(&[unsound::label::new("banana.rs")]),
                diff: FileDiff::Plain {
                    hunks: Hunks::default(),
                },
                eof: None,
            }],
        };

        assert_eq!(diff, expected_diff)
    }

    #[test]
    fn test_create_directory() {
        let directory = Directory::root();

        let mut new_directory = Directory::root();
        new_directory.insert_file(
            unsound::path::new("src/banana.rs"),
            File::new(b"use banana"),
        );

        let diff = Diff::diff(directory, new_directory);

        let expected_diff = Diff {
            created: vec![CreateFile {
                path: Path::with_root(&[
                    unsound::label::new("src"),
                    unsound::label::new("banana.rs"),
                ]),
                diff: FileDiff::Plain {
                    hunks: Hunks::default(),
                },
            }],
            deleted: vec![],
            moved: vec![],
            copied: vec![],
            modified: vec![],
        };

        assert_eq!(diff, expected_diff)
    }

    #[test]
    fn test_delete_directory() {
        let mut directory = Directory::root();
        directory.insert_file(
            unsound::path::new("src/banana.rs"),
            File::new(b"use banana"),
        );

        let new_directory = Directory::root();

        let diff = Diff::diff(directory, new_directory);

        let expected_diff = Diff {
            created: vec![],
            deleted: vec![DeleteFile {
                path: Path::with_root(&[
                    unsound::label::new("src"),
                    unsound::label::new("banana.rs"),
                ]),
                diff: FileDiff::Plain {
                    hunks: Hunks::default(),
                },
            }],
            moved: vec![],
            copied: vec![],
            modified: vec![],
        };

        assert_eq!(diff, expected_diff)
    }

    #[test]
    fn test_modify_file_directory() {
        let mut directory = Directory::root();
        directory.insert_file(
            unsound::path::new("src/banana.rs"),
            File::new(b"use banana"),
        );

        let mut new_directory = Directory::root();
        new_directory.insert_file(
            unsound::path::new("src/banana.rs"),
            File::new(b"use banana;"),
        );

        let diff = Diff::diff(directory, new_directory);

        let expected_diff = Diff {
            created: vec![],
            deleted: vec![],
            moved: vec![],
            copied: vec![],
            modified: vec![ModifiedFile {
                path: Path::with_root(&[
                    unsound::label::new("src"),
                    unsound::label::new("banana.rs"),
                ]),
                diff: FileDiff::Plain {
                    hunks: Hunks::default(),
                },
                eof: None,
            }],
        };

        assert_eq!(diff, expected_diff)
    }

    /* TODO(fintan): Tricky stuff
    #[test]
    fn test_disjoint_directories() {
        let mut directory = Directory::root();
        directory.insert_file(
            &unsound::path::new("foo/src/banana.rs"),
            File::new(b"use banana"),
        );

        let mut other_directory = Directory::root();
        other_directory.insert_file(
            &unsound::path::new("bar/src/pineapple.rs"),
            File::new(b"use pineapple"),
        );

        let diff = Diff::diff(directory, other_directory).expect("diff failed");

        let expected_diff = Diff {
            created: vec![CreateFile(Path::from_labels(
                unsound::label::new("bar"),
                &[
                    unsound::label::new("src"),
                    unsound::label::new("pineapple.rs"),
                ],
            ))],
            deleted: vec![DeleteFile(Path::from_labels(
                unsound::label::new("foo"),
                &[unsound::label::new("src"), unsound::label::new("banana.rs")],
            ))],
            moved: vec![],
            modified: vec![],
        };

        assert_eq!(diff, expected_diff)
    }
    */
}
