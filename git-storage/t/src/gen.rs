// Copyright © 2022 The Radicle Link Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

use git_storage::{
    odb::{self, write::Write as _},
    signature::UserInfo,
    Write,
};
use proptest::prelude::*;
use radicle_git_ext::Oid;

use git2::FileMode;

/// Represents a file in the git tree but without linking it to the repo yet
#[derive(Clone, Debug)]
pub struct File {
    pub path: PathBuf,
    pub inner: Vec<u8>,
    pub mode: git2::FileMode,
    pub oid: git2::Oid,
}

/// Represents a Tree to be written with the ODB writer.
///
/// This Tree does not have an explicit link to a repository, linking is
/// performed by writing it to the repo using the ODB writer.
///
/// Used as a replacement of git2::TreeBuilder, which is more complicated to
/// build since it requires a repository from the beginning.
#[derive(Clone, Debug)]
pub struct Tree {
    builder: odb::TreeBuilder,
    files: Vec<File>,
}

impl Tree {
    /// Write the files to the filesystem and the repository `storage`
    pub fn write(self, storage: &Write) -> Result<Oid, git2::Error> {
        self.write_files(storage)?;
        storage.write_tree(self.builder)
    }

    /// Write the files to the filesystem
    pub fn write_files(&self, storage: &Write) -> Result<(), git2::Error> {
        for file in &self.files {
            storage.write_blob(&file.inner)?;
        }
        Ok(())
    }
}

/// Any valid filename
pub fn trivial() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9]+"
}

pub fn gen_signature() -> impl Strategy<Value = UserInfo> {
    trivial().prop_map(move |name| {
        UserInfo {
            name: name.clone(),
            // TODO: is it worth to make this more realistic?
            email: format!("{}@{}.com", &name, &name),
        }
    })
}

pub fn gen_bytes() -> impl Strategy<Value = Vec<u8>> {
    any::<Vec<u8>>()
}

pub fn gen_mode() -> impl Strategy<Value = FileMode> {
    prop_oneof![Just(FileMode::Blob), Just(FileMode::BlobExecutable)]
}

prop_compose! {
    pub fn gen_file()
                    (path in trivial(),
                     blob in gen_bytes(),
                     mode in gen_mode())
                    -> File {
        let oid = git2::Oid::hash_object(git2::ObjectType::Blob, &blob).unwrap();
        File {
            path: PathBuf::from(path),
            inner: blob,
            mode,
            oid,
        }
    }
}

pub fn gen_file_set(max_size: u8) -> impl Strategy<Value = Vec<File>> {
    prop::collection::vec(gen_file(), 0..(max_size as usize))
}

/// Generates a [`odb::TreeBuilder`] and a set of [`File`]s.
///
/// To write the resulting `Tree`, you MUST write the [`File::oid`] to the
/// repository first.
pub fn gen_tree(max_size: u8) -> impl Strategy<Value = Tree> {
    gen_file_set(max_size).prop_map(move |files| {
        let builder = files.iter().fold(odb::TreeBuilder::new(), |tree, file| {
            tree.insert(file.path.clone(), file.oid.into(), file.mode)
        });
        Tree { builder, files }
    })
}
