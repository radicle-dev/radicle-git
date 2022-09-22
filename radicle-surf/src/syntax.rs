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

use std::path;

use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

lazy_static::lazy_static! {
    // The syntax set is slow to load (~30ms), so we make sure to only load it once.
    // It _will_ affect the latency of the first request that uses syntax highlighting,
    // but this is acceptable for now.
    pub static ref SYNTAX_SET: SyntaxSet = {
        let default_set = SyntaxSet::load_defaults_newlines();
        let mut builder = default_set.into_builder();

        if cfg!(debug_assertions) {
            // In development assets are relative to the proxy source.
            // Don't crash if we aren't able to load additional syntaxes for some reason.
            builder.add_from_folder("./assets", true).ok();
        } else {
            // In production assets are relative to the proxy executable.
            let exe_path = std::env::current_exe().expect("Can't get current exe path");
            let root_path = exe_path
                .parent()
                .expect("Could not get parent path of current executable");
            let mut tmp = root_path.to_path_buf();
            tmp.push("assets");
            let asset_path = tmp.to_str().expect("Couldn't convert pathbuf to str");

            // Don't crash if we aren't able to load additional syntaxes for some reason.
            match builder.add_from_folder(asset_path, true) {
                Ok(_) => (),
                Err(err) => log::warn!("Syntax builder error : {}", err),
            };
        }
        builder.build()
    };
}

/// Return a [`BlobContent`] given a file path, content and theme. Attempts to
/// perform syntax highlighting when the theme is `Some`.
pub fn highlight(path: &str, content: &str, theme_name: &str) -> Option<String> {
    let syntax = path::Path::new(path)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .and_then(|ext| SYNTAX_SET.find_syntax_by_extension(ext));

    let ts = ThemeSet::load_defaults();
    let theme = ts.themes.get(theme_name);

    match (syntax, theme) {
        (Some(syntax), Some(theme)) => {
            let mut highlighter = HighlightLines::new(syntax, theme);
            let mut html = String::with_capacity(content.len());

            for line in LinesWithEndings::from(content) {
                let regions = highlighter.highlight(line, &SYNTAX_SET);
                syntect::html::append_highlighted_html_for_styled_line(
                    &regions[..],
                    syntect::html::IncludeBackground::No,
                    &mut html,
                );
            }
            Some(html)
        },
        _ => None,
    }
}
