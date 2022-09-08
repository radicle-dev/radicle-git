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

//! In Irish slang there exists the term "sound". One is a "sound" person if
//! they are nice and you can rely on them. This module is the anithesis of
//! being "sound", you might say it is "unsound".
//!
//! The aim of this module is to make testing easier. During test time, _we
//! know_ that a string is going to be non-empty because we are using the
//! literal `"sound_label"`. The same for knowing that the form
//! `"what/a/sound/bunch"` is a valid path.
//!
//! On the other hand, if we do not control the data coming in we should use the
//! more "sound" method of the [`std::convert::TryFrom`] instance for
//! [`crate::file_system::Label`] and [`crate::file_system::Path`]
//! to ensure we have valid data to use for further operations.

pub mod path {
    //! Unsound creation of [`Path`]s.

    use crate::file_system::path::Path;
    use std::convert::TryFrom;

    /// **NB**: Use with caution!
    ///
    /// Calls `try_from` on the input and expects it to not fail.
    ///
    /// Used for testing and playground purposes.
    pub fn new(path: &str) -> Path {
        Path::try_from(path).expect("unsafe_path: Failed to parse path")
    }
}

pub mod label {
    //! Unsound creation of [`Label`]s.

    use crate::file_system::path::Label;
    use std::convert::TryFrom;

    /// **NB**: Use with caution!
    ///
    /// Calls `try_from` on the input and expects it to not fail.
    ///
    /// Used for testing and playground purposes.
    pub fn new(path: &str) -> Label {
        Label::try_from(path).expect("unsafe_path: Failed to parse label")
    }
}
