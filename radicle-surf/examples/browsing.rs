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

//! An example of browsing a git repo using `radicle-surf`.
//!
//! How to run:
//!
//!     cargo run --example browsing <git_repo_path>
//!
//! This program browses the given repo and prints out the files and
//! the directories in a tree-like structure.

use radicle_surf::{
    file_system::{Directory, DirectoryEntry},
    git::{Repository, RepositoryRef},
};
use std::{env, time::Instant};

fn main() {
    let repo_path = match env::args().nth(1) {
        Some(path) => path,
        None => {
            print_usage();
            return;
        },
    };
    let repo = Repository::discover(&repo_path).unwrap();
    let repo = repo.as_ref();
    let now = Instant::now();
    let head = repo.head_oid().unwrap();
    let root = repo.root_dir(head).unwrap();
    print_directory(&root, &repo, 0);

    let elapsed_millis = now.elapsed().as_millis();
    println!("browse with print: {} ms", elapsed_millis);
}

fn print_directory(d: &Directory, repo: &RepositoryRef, indent_level: usize) {
    let indent = " ".repeat(indent_level * 4);
    println!("{}{}/", &indent, d.name());
    for entry in d.contents(repo).unwrap().iter() {
        match entry {
            DirectoryEntry::File(f) => println!("    {}{}", &indent, f.name()),
            DirectoryEntry::Directory(d) => print_directory(d, repo, indent_level + 1),
        }
    }
}

fn print_usage() {
    println!("Usage:");
    println!("cargo run --example browsing <repo_path>");
}
