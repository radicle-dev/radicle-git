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

use std::{
    env, fs,
    fs::File,
    io,
    path::{Path, PathBuf},
};

use anyhow::Context as _;
use flate2::read::GzDecoder;
use tar::Archive;

enum Command {
    Build(PathBuf),
    Publish(PathBuf),
}

impl Command {
    fn new() -> io::Result<Self> {
        let current = env::current_dir()?;
        Ok(if current.ends_with("radicle-surf") {
            Self::Build(current)
        } else {
            Self::Publish(PathBuf::from(
                env::var("OUT_DIR").map_err(|err| io::Error::new(io::ErrorKind::Other, err))?,
            ))
        })
    }

    fn target(&self) -> PathBuf {
        match self {
            Self::Build(path) => path.join("data"),
            Self::Publish(path) => path.join("data"),
        }
    }
}

fn main() {
    let target = Command::new()
        .expect("could not determine the cargo command")
        .target();
    let git_platinum_tarball = "./data/git-platinum.tgz";

    unpack(git_platinum_tarball, target).expect("Failed to unpack git-platinum");

    println!("cargo:rerun-if-changed={git_platinum_tarball}");
}

fn unpack(archive_path: impl AsRef<Path>, target: impl AsRef<Path>) -> anyhow::Result<()> {
    let content = target.as_ref().join("git-platinum");
    if content.exists() {
        fs::remove_dir_all(content).context("attempting to remove git-platinum")?;
    }
    let archive_path = archive_path.as_ref();
    let tar_gz = File::open(archive_path).context(format!(
        "attempting to open file: {}",
        archive_path.display()
    ))?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(target).context("attempting to unpack")?;

    Ok(())
}
