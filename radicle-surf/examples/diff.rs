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

extern crate radicle_surf;

use std::{env::Args, str::FromStr, time::Instant};

use nonempty::NonEmpty;

use radicle_git_ext::Oid;
use radicle_surf::{
    diff::Diff,
    file_system::Directory,
    vcs::{git, History},
};

fn main() {
    let options = get_options_or_exit();
    let repo = init_repository_or_exit(&options.path_to_repo);
    let mut browser =
        git::Browser::new(&repo, git::Branch::local("master")).expect("failed to create browser:");

    match options.head_revision {
        HeadRevision::Head => {
            reset_browser_to_head_or_exit(&mut browser);
        },
        HeadRevision::Commit(id) => {
            set_browser_history_or_exit(&mut browser, &id);
        },
    }
    let head_directory = get_directory_or_exit(&browser);

    set_browser_history_or_exit(&mut browser, &options.base_revision);
    let base_directory = get_directory_or_exit(&browser);

    let now = Instant::now();
    let elapsed_nanos = now.elapsed().as_nanos();
    let diff = Diff::diff(base_directory, head_directory);
    print_diff_summary(&diff, elapsed_nanos);
}

fn get_options_or_exit() -> Options {
    match Options::parse(std::env::args()) {
        Ok(options) => options,
        Err(message) => {
            println!("{}", message);
            std::process::exit(1);
        },
    }
}

fn init_repository_or_exit(path_to_repo: &str) -> git::Repository {
    match git::Repository::new(path_to_repo) {
        Ok(repo) => repo,
        Err(e) => {
            println!("Failed to create repository: {:?}", e);
            std::process::exit(1);
        },
    }
}

fn reset_browser_to_head_or_exit(browser: &mut git::Browser) {
    if let Err(e) = browser.head() {
        println!("Failed to set browser to HEAD: {:?}", e);
        std::process::exit(1);
    }
}

fn set_browser_history_or_exit(browser: &mut git::Browser, commit_id: &str) {
    // TODO: Might consider to not require resetting to HEAD when history is not at
    // HEAD
    reset_browser_to_head_or_exit(browser);
    if let Err(e) = set_browser_history(browser, commit_id) {
        println!("Failed to set browser history: {:?}", e);
        std::process::exit(1);
    }
}

fn set_browser_history(browser: &mut git::Browser, commit_id: &str) -> Result<(), String> {
    let oid = match Oid::from_str(commit_id) {
        Ok(oid) => oid,
        Err(e) => return Err(format!("{}", e)),
    };
    let commit = match browser.get().find_in_history(&oid, |artifact| artifact.id) {
        Some(commit) => commit,
        None => return Err(format!("Git commit not found: {}", commit_id)),
    };
    browser.set(History(NonEmpty::new(commit)));
    Ok(())
}

fn get_directory_or_exit(browser: &git::Browser) -> Directory {
    match browser.get_directory() {
        Ok(dir) => dir,
        Err(e) => {
            println!("Failed to get directory: {:?}", e);
            std::process::exit(1)
        },
    }
}

fn print_diff_summary(diff: &Diff, elapsed_nanos: u128) {
    diff.created.iter().for_each(|created| {
        println!("+++ {}", created.path);
    });
    diff.deleted.iter().for_each(|deleted| {
        println!("--- {}", deleted.path);
    });
    diff.modified.iter().for_each(|modified| {
        println!("mod {}", modified.path);
    });

    println!(
        "created {} / deleted {} / modified {} / total {}",
        diff.created.len(),
        diff.deleted.len(),
        diff.modified.len(),
        diff.created.len() + diff.deleted.len() + diff.modified.len()
    );
    println!("diff took {} micros ", elapsed_nanos / 1000);
}

struct Options {
    path_to_repo: String,
    base_revision: String,
    head_revision: HeadRevision,
}

enum HeadRevision {
    Head,
    Commit(String),
}

impl Options {
    fn parse(args: Args) -> Result<Self, String> {
        let args: Vec<String> = args.collect();
        if args.len() != 4 {
            return Err(format!(
                "Usage: {} <path-to-repo> <base-revision> <head-revision>\n\
                \tpath-to-repo: Path to the directory containing .git subdirectory\n\
                \tbase-revision: Git commit ID of the base revision (one that will be considered less recent)\n\
                \thead-revision: Git commit ID of the head revision (one that will be considered more recent) or 'HEAD' to use current git HEAD\n",
                args[0]));
        }

        let path_to_repo = args[1].clone();
        let base_revision = args[2].clone();
        let head_revision = {
            if args[3].eq_ignore_ascii_case("HEAD") {
                HeadRevision::Head
            } else {
                HeadRevision::Commit(args[3].clone())
            }
        };

        Ok(Options {
            path_to_repo,
            base_revision,
            head_revision,
        })
    }
}
