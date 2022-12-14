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

use std::convert::TryFrom;

use crate::diff::{self, Addition, Deletion, Diff, EofNewLine, Hunk, Hunks, Line, Modification};

pub mod error {
    use std::path::PathBuf;

    use thiserror::Error;

    #[derive(Debug, Error)]
    #[non_exhaustive]
    pub enum Addition {
        #[error(transparent)]
        Git(#[from] git2::Error),
        #[error("the new line number was missing for an added line")]
        MissingNewLineNo,
    }

    #[derive(Debug, Error)]
    #[non_exhaustive]
    pub enum Deletion {
        #[error(transparent)]
        Git(#[from] git2::Error),
        #[error("the new line number was missing for an deleted line")]
        MissingOldLineNo,
    }

    #[derive(Debug, Error)]
    #[non_exhaustive]
    pub enum Modification {
        /// A Git `DiffLine` is invalid.
        #[error(
            "invalid `git2::DiffLine` which contains no line numbers for either side of the diff"
        )]
        Invalid,
    }

    #[derive(Debug, Error)]
    #[non_exhaustive]
    pub enum Hunk {
        #[error(transparent)]
        Git(#[from] git2::Error),
        #[error(transparent)]
        Line(#[from] Modification),
    }

    /// A Git diff error.
    #[derive(Debug, Error)]
    #[non_exhaustive]
    pub enum Diff {
        #[error(transparent)]
        Addition(#[from] Addition),
        #[error(transparent)]
        Deletion(#[from] Deletion),
        /// A Git delta type isn't currently handled.
        #[error("git delta type is not handled")]
        DeltaUnhandled(git2::Delta),
        #[error(transparent)]
        Git(#[from] git2::Error),
        #[error(transparent)]
        Hunk(#[from] Hunk),
        #[error(transparent)]
        Line(#[from] Modification),
        /// A patch is unavailable.
        #[error("couldn't retrieve patch for {0}")]
        PatchUnavailable(PathBuf),
        /// A The path of a file isn't available.
        #[error("couldn't retrieve file path")]
        PathUnavailable,
    }
}

impl TryFrom<git2::Patch<'_>> for Hunks<Modification> {
    type Error = error::Hunk;

    fn try_from(patch: git2::Patch) -> Result<Self, Self::Error> {
        let mut hunks = Vec::new();
        for h in 0..patch.num_hunks() {
            let (hunk, hunk_lines) = patch.hunk(h)?;
            let header = Line(hunk.header().to_owned());
            let mut lines: Vec<Modification> = Vec::new();

            for l in 0..hunk_lines {
                let line = patch.line_in_hunk(h, l)?;
                let line = Modification::try_from(line)?;
                lines.push(line);
            }
            hunks.push(Hunk { header, lines });
        }
        Ok(Hunks(hunks))
    }
}

impl TryFrom<git2::Patch<'_>> for Hunks<Addition> {
    type Error = error::Addition;

    fn try_from(patch: git2::Patch) -> Result<Self, Self::Error> {
        let mut hunks = Vec::with_capacity(patch.num_hunks());
        for h in 0..patch.num_hunks() {
            let (hunk, hunk_lines) = patch.hunk(h)?;
            let header = Line(hunk.header().to_owned());
            let mut lines: Vec<Addition> = Vec::with_capacity(hunk_lines);

            for l in 0..hunk_lines {
                let line = patch.line_in_hunk(h, l)?;
                lines.push(Addition::try_from(line)?);
            }
            hunks.push(Hunk { header, lines })
        }
        Ok(hunks.into())
    }
}

impl TryFrom<git2::Patch<'_>> for Hunks<Deletion> {
    type Error = error::Deletion;

    fn try_from(patch: git2::Patch) -> Result<Self, Self::Error> {
        let mut hunks = Vec::with_capacity(patch.num_hunks());
        for h in 0..patch.num_hunks() {
            let (hunk, hunk_lines) = patch.hunk(h)?;
            let header = Line(hunk.header().to_owned());
            let mut lines: Vec<Deletion> = Vec::with_capacity(hunk_lines);

            for l in 0..hunk_lines {
                let line = patch.line_in_hunk(h, l)?;
                lines.push(Deletion::try_from(line)?);
            }
            hunks.push(Hunk { header, lines })
        }
        Ok(hunks.into())
    }
}

impl<'a> TryFrom<git2::DiffLine<'a>> for Modification {
    type Error = error::Modification;

    fn try_from(line: git2::DiffLine) -> Result<Self, Self::Error> {
        match (line.old_lineno(), line.new_lineno()) {
            (None, Some(n)) => Ok(Self::addition(line.content().to_owned(), n)),
            (Some(n), None) => Ok(Self::deletion(line.content().to_owned(), n)),
            (Some(l), Some(r)) => Ok(Self::context(line.content().to_owned(), l, r)),
            (None, None) => Err(error::Modification::Invalid),
        }
    }
}

impl TryFrom<git2::DiffLine<'_>> for Addition {
    type Error = error::Addition;

    fn try_from(line: git2::DiffLine) -> Result<Self, Self::Error> {
        debug_assert!(
            line.old_lineno().is_none(),
            "trying to build Addition with a modified line"
        );
        Ok(Self {
            line: line.content().to_owned().into(),
            line_no: line.new_lineno().ok_or(error::Addition::MissingNewLineNo)?,
        })
    }
}

impl TryFrom<git2::DiffLine<'_>> for Deletion {
    type Error = error::Deletion;

    fn try_from(line: git2::DiffLine) -> Result<Self, Self::Error> {
        debug_assert!(
            line.new_lineno().is_none(),
            "trying to build Deletion with a modified line"
        );
        Ok(Self {
            line: line.content().to_owned().into(),
            line_no: line.old_lineno().ok_or(error::Deletion::MissingOldLineNo)?,
        })
    }
}

impl From<git2::DiffStats> for diff::Stats {
    fn from(stats: git2::DiffStats) -> Self {
        Self {
            files_changed: stats.files_changed(),
            insertions: stats.insertions(),
            deletions: stats.deletions(),
        }
    }
}

impl<'a> TryFrom<git2::Diff<'a>> for Diff {
    type Error = error::Diff;

    fn try_from(git_diff: git2::Diff) -> Result<Diff, Self::Error> {
        use git2::Delta;

        let mut diff = Diff::new();
        diff.stats = git_diff.stats()?.into();

        for (idx, delta) in git_diff.deltas().enumerate() {
            match delta.status() {
                Delta::Added => created(&mut diff, &git_diff, idx, &delta)?,
                Delta::Deleted => deleted(&mut diff, &git_diff, idx, &delta)?,
                Delta::Modified => modified(&mut diff, &git_diff, idx, &delta)?,
                Delta::Renamed => renamed(&mut diff, &delta)?,
                Delta::Copied => copied(&mut diff, &delta)?,
                status => {
                    return Err(error::Diff::DeltaUnhandled(status));
                },
            }
        }

        Ok(diff)
    }
}

fn created(
    diff: &mut Diff,
    git_diff: &git2::Diff<'_>,
    idx: usize,
    delta: &git2::DiffDelta<'_>,
) -> Result<(), error::Diff> {
    let diff_file = delta.new_file();
    let path = diff_file
        .path()
        .ok_or(error::Diff::PathUnavailable)?
        .to_path_buf();

    let patch = git2::Patch::from_diff(git_diff, idx)?;
    if let Some(patch) = patch {
        diff.added(
            path,
            diff::FileDiff::Plain {
                hunks: Hunks::try_from(patch)?,
            },
        );
    } else {
        diff.added(
            path,
            diff::FileDiff::Plain {
                hunks: Hunks::default(),
            },
        );
    }
    Ok(())
}

fn deleted(
    diff: &mut Diff,
    git_diff: &git2::Diff<'_>,
    idx: usize,
    delta: &git2::DiffDelta<'_>,
) -> Result<(), error::Diff> {
    let diff_file = delta.old_file();
    let path = diff_file
        .path()
        .ok_or(error::Diff::PathUnavailable)?
        .to_path_buf();
    let patch = git2::Patch::from_diff(git_diff, idx)?;
    if let Some(patch) = patch {
        diff.deleted(
            path,
            diff::FileDiff::Plain {
                hunks: Hunks::try_from(patch)?,
            },
        );
    } else {
        diff.deleted(
            path,
            diff::FileDiff::Plain {
                hunks: Hunks::default(),
            },
        );
    }
    Ok(())
}

fn modified(
    diff: &mut Diff,
    git_diff: &git2::Diff<'_>,
    idx: usize,
    delta: &git2::DiffDelta<'_>,
) -> Result<(), error::Diff> {
    let diff_file = delta.new_file();
    let path = diff_file
        .path()
        .ok_or(error::Diff::PathUnavailable)?
        .to_path_buf();
    let patch = git2::Patch::from_diff(git_diff, idx)?;

    if let Some(patch) = patch {
        let mut hunks: Vec<Hunk<Modification>> = Vec::new();
        let mut old_missing_eof = false;
        let mut new_missing_eof = false;

        for h in 0..patch.num_hunks() {
            let (hunk, hunk_lines) = patch.hunk(h)?;
            let header = Line(hunk.header().to_owned());
            let mut lines: Vec<Modification> = Vec::new();

            for l in 0..hunk_lines {
                let line = patch.line_in_hunk(h, l)?;
                match line.origin_value() {
                    git2::DiffLineType::ContextEOFNL => {
                        new_missing_eof = true;
                        old_missing_eof = true;
                        continue;
                    },
                    git2::DiffLineType::AddEOFNL => {
                        old_missing_eof = true;
                        continue;
                    },
                    git2::DiffLineType::DeleteEOFNL => {
                        new_missing_eof = true;
                        continue;
                    },
                    _ => {},
                }
                let line = Modification::try_from(line)?;
                lines.push(line);
            }
            hunks.push(Hunk { header, lines });
        }
        let eof = match (old_missing_eof, new_missing_eof) {
            (true, true) => Some(EofNewLine::BothMissing),
            (true, false) => Some(EofNewLine::OldMissing),
            (false, true) => Some(EofNewLine::NewMissing),
            (false, false) => None,
        };
        diff.modified(path, hunks, eof);
        Ok(())
    } else if diff_file.is_binary() {
        diff.modified_binary(path);
        Ok(())
    } else {
        Err(error::Diff::PatchUnavailable(path))
    }
}

fn renamed(diff: &mut Diff, delta: &git2::DiffDelta<'_>) -> Result<(), error::Diff> {
    let old = delta
        .old_file()
        .path()
        .ok_or(error::Diff::PathUnavailable)?;
    let new = delta
        .new_file()
        .path()
        .ok_or(error::Diff::PathUnavailable)?;

    diff.moved(old.to_path_buf(), new.to_path_buf());
    Ok(())
}

fn copied(diff: &mut Diff, delta: &git2::DiffDelta<'_>) -> Result<(), error::Diff> {
    let old = delta
        .old_file()
        .path()
        .ok_or(error::Diff::PathUnavailable)?;
    let new = delta
        .new_file()
        .path()
        .ok_or(error::Diff::PathUnavailable)?;

    diff.copied(old.to_path_buf(), new.to_path_buf());
    Ok(())
}
