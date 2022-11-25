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

use radicle_git_ext::Oid;
use radicle_surf::{diff::Diff, git};

fn main() {
    let options = get_options_or_exit();
    let repo = init_repository_or_exit(&options.path_to_repo);
    let head_oid = match options.head_revision {
        HeadRevision::Head => repo.head_oid().unwrap(),
        HeadRevision::Commit(id) => Oid::from_str(&id).unwrap(),
    };
    let base_oid = Oid::from_str(&options.base_revision).unwrap();
    let now = Instant::now();
    let elapsed_nanos = now.elapsed().as_nanos();
    let diff = repo.diff(base_oid, head_oid).unwrap();
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
    match git::Repository::open(path_to_repo) {
        Ok(repo) => repo,
        Err(e) => {
            println!("Failed to create repository: {:?}", e);
            std::process::exit(1);
        },
    }
}

fn print_diff_summary(diff: &Diff, elapsed_nanos: u128) {
    diff.added.iter().for_each(|created| {
        println!("+++ {:?}", created.path);
    });
    diff.deleted.iter().for_each(|deleted| {
        println!("--- {:?}", deleted.path);
    });
    diff.modified.iter().for_each(|modified| {
        println!("mod {:?}", modified.path);
    });

    println!(
        "created {} / deleted {} / modified {} / total {}",
        diff.added.len(),
        diff.deleted.len(),
        diff.modified.len(),
        diff.added.len() + diff.deleted.len() + diff.modified.len()
    );
    println!("diff took {} nanos ", elapsed_nanos);
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
