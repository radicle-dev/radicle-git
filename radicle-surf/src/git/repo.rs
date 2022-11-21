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
    collections::{BTreeMap, BTreeSet},
    convert::TryFrom,
    path::PathBuf,
    str,
};

use directory::{Directory, FileContent};
use git_ref_format::{refspec::QualifiedPattern, Qualified};
use radicle_git_ext::Oid;
use thiserror::Error;

use crate::{
    diff::{self, *},
    file_system,
    file_system::{directory, DirectoryEntry, Label},
    git::{
        commit,
        glob,
        namespace,
        Branch,
        Commit,
        Glob,
        History,
        Namespace,
        Revision,
        Signature,
        Stats,
        Tag,
        ToCommit,
    },
};

pub mod iter;
pub use iter::{Branches, Namespaces, Tags};

use self::iter::{BranchNames, TagNames};

/// Enumeration of errors that can occur in operations from [`crate::git`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Branches(#[from] iter::error::Branch),
    #[error(transparent)]
    Commit(#[from] commit::Error),
    /// An error that comes from performing a *diff* operations.
    #[error(transparent)]
    Diff(#[from] diff::git::error::Diff),
    /// An error that comes from performing a [`crate::file_system`] operation.
    #[error(transparent)]
    FileSystem(#[from] file_system::Error),
    /// A wrapper around the generic [`git2::Error`].
    #[error(transparent)]
    Git(#[from] git2::Error),
    #[error(transparent)]
    Glob(#[from] glob::Error),
    #[error(transparent)]
    Namespace(#[from] namespace::Error),
    #[error("the reference '{0}' should be of the form 'refs/<category>/<path>'")]
    NotQualified(String),
    /// The requested file was not found.
    #[error("path not found for: {0}")]
    PathNotFound(file_system::Path),
    #[error(transparent)]
    RefFormat(#[from] git_ref_format::Error),
    #[error(transparent)]
    Revision(Box<dyn std::error::Error + Send + Sync + 'static>),
    /// A `revspec` was provided that could not be parsed into a branch, tag, or
    /// commit object.
    #[error("provided revspec '{rev}' could not be parsed into a git object")]
    RevParseFailure {
        /// The provided revspec that failed to parse.
        rev: String,
    },
    #[error(transparent)]
    ToCommit(Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error(transparent)]
    Tags(#[from] iter::error::Tag),
}

/// Wrapper around the `git2`'s `git2::Repository` type.
/// This is to to limit the functionality that we can do
/// on the underlying object.
pub struct Repository {
    repo: git2::Repository,
}

impl Repository {
    /// What is the current namespace we're browsing in.
    pub fn which_namespace(&self) -> Result<Option<Namespace>, Error> {
        self.repo
            .namespace_bytes()
            .map(|ns| Namespace::try_from(ns).map_err(Error::from))
            .transpose()
    }

    /// Returns an iterator of branches that match `pattern`.
    pub fn branches(&self, pattern: &Glob<Branch>) -> Result<Branches, Error> {
        let mut branches = Branches::default();
        for glob in pattern.globs() {
            let namespaced = self.namespaced_pattern(glob)?;
            let references = self.repo.references_glob(&namespaced)?;
            branches.push(references);
        }
        Ok(branches)
    }

    /// Returns an iterator of tags that match `pattern`.
    pub fn tags(&self, pattern: &Glob<Tag>) -> Result<Tags, Error> {
        let mut tags = Tags::default();
        for glob in pattern.globs() {
            let namespaced = self.namespaced_pattern(glob)?;
            let references = self.repo.references_glob(&namespaced)?;
            tags.push(references);
        }
        Ok(tags)
    }

    /// Returns an iterator of namespaces that match `pattern`.
    pub fn namespaces(&self, pattern: &Glob<Namespace>) -> Result<Namespaces, Error> {
        let mut set = BTreeSet::new();
        for glob in pattern.globs() {
            let new_set = self
                .repo
                .references_glob(glob)?
                .map(|reference| {
                    reference
                        .map_err(Error::Git)
                        .and_then(|r| Namespace::try_from(&r).map_err(Error::from))
                })
                .collect::<Result<BTreeSet<Namespace>, Error>>()?;
            set.extend(new_set);
        }
        Ok(Namespaces::new(set))
    }

    /// Get the [`Diff`] between two commits.
    pub fn diff(&self, from: impl Revision, to: impl Revision) -> Result<Diff, Error> {
        let from_commit = self.get_git2_commit(self.object_id(&from)?)?;
        let to_commit = self.get_git2_commit(self.object_id(&to)?)?;
        self.diff_commits(None, Some(&from_commit), &to_commit)
            .and_then(|diff| Diff::try_from(diff).map_err(Error::from))
    }

    /// Get the [`Diff`] of a commit with no parents.
    pub fn initial_diff<R: Revision>(&self, rev: R) -> Result<Diff, Error> {
        let commit = self.get_git2_commit(self.object_id(&rev)?)?;
        self.diff_commits(None, None, &commit)
            .and_then(|diff| Diff::try_from(diff).map_err(Error::from))
    }

    /// Get the diff introduced by a particlar rev.
    pub fn diff_from_parent<C: ToCommit>(&self, commit: C) -> Result<Diff, Error> {
        let commit = commit
            .to_commit(self)
            .map_err(|err| Error::ToCommit(err.into()))?;
        match commit.parents.first() {
            Some(parent) => self.diff(*parent, commit.id),
            None => self.initial_diff(commit.id),
        }
    }

    /// Parse an [`Oid`] from the given string.
    pub fn oid(&self, oid: &str) -> Result<Oid, Error> {
        Ok(self.repo.revparse_single(oid)?.id().into())
    }

    /// Returns a top level `Directory` without nested sub-directories.
    ///
    /// To visit inside any nested sub-directories, call `directory.get(&repo)`
    /// on the sub-directory.
    pub fn root_dir<C: ToCommit>(&self, commit: C) -> Result<Directory, Error> {
        let commit = commit
            .to_commit(self)
            .map_err(|err| Error::ToCommit(err.into()))?;
        let git2_commit = self.repo.find_commit((commit.id).into())?;
        let tree = git2_commit.as_object().peel_to_tree()?;
        Ok(Directory {
            name: Label::root(),
            oid: tree.id().into(),
        })
    }

    /// Retrieves the content of a directory.
    pub(crate) fn directory_get(
        &self,
        d: &Directory,
    ) -> Result<BTreeMap<Label, DirectoryEntry>, Error> {
        let git2_tree = self.repo.find_tree(d.oid.into())?;
        let map = self.tree_first_level(git2_tree)?;
        Ok(map)
    }

    /// Returns a map of the first level entries in `tree`.
    fn tree_first_level(&self, tree: git2::Tree) -> Result<BTreeMap<Label, DirectoryEntry>, Error> {
        let mut map = BTreeMap::new();

        // Walks only the first level of entries.
        tree.walk(git2::TreeWalkMode::PreOrder, |_s, entry| {
            let oid = entry.id().into();
            let label = match entry.name() {
                Some(name) => match name.parse::<Label>() {
                    Ok(label) => label,
                    Err(_) => return git2::TreeWalkResult::Abort,
                },
                None => return git2::TreeWalkResult::Abort,
            };

            match entry.kind() {
                Some(git2::ObjectType::Tree) => {
                    let dir = Directory::new(label.clone(), oid);
                    map.insert(label, DirectoryEntry::Directory(dir));
                    return git2::TreeWalkResult::Skip; // Not go into nested
                                                       // directories.
                },
                Some(git2::ObjectType::Blob) => {
                    let f = directory::File {
                        name: label.clone(),
                        oid,
                    };
                    map.insert(label, DirectoryEntry::File(f));
                },
                _ => {
                    return git2::TreeWalkResult::Skip;
                },
            }

            git2::TreeWalkResult::Ok
        })?;

        Ok(map)
    }

    /// Returns the last commit, if exists, for a `path` in the history of
    /// `rev`.
    pub fn last_commit<C: ToCommit>(
        &self,
        path: file_system::Path,
        rev: C,
    ) -> Result<Option<Commit>, Error> {
        let history = self.history(rev)?;
        history.by_path(path).next().transpose()
    }

    /// Returns a commit for `rev` if exists.
    pub fn commit<R: Revision>(&self, rev: R) -> Result<Commit, Error> {
        rev.to_commit(self)
    }

    /// Gets stats of `commit`.
    pub fn get_commit_stats<C: ToCommit>(&self, commit: C) -> Result<Stats, Error> {
        let branches = self.branches(&Glob::heads("*")?)?.count();
        let history = self.history(commit)?;
        let mut commits = 0;

        let contributors = history
            .filter_map(|commit| match commit {
                Ok(commit) => {
                    commits += 1;
                    Some((commit.author.name, commit.author.email))
                },
                Err(_) => None,
            })
            .collect::<BTreeSet<_>>();

        Ok(Stats {
            branches,
            commits,
            contributors: contributors.len(),
        })
    }

    /// Obtain the file content
    pub(crate) fn file_content(&self, object_id: Oid) -> Result<FileContent, Error> {
        let blob = self.repo.find_blob(object_id.into())?;
        Ok(FileContent::new(blob))
    }

    /// Return the size of a file
    pub(crate) fn file_size(&self, oid: Oid) -> Result<usize, Error> {
        let blob = self.repo.find_blob(oid.into())?;
        Ok(blob.size())
    }

    /// Retrieves the file with `path` in this commit.
    pub fn get_commit_file<R: Revision>(
        &self,
        rev: &R,
        path: file_system::Path,
    ) -> Result<FileContent, crate::git::Error> {
        let id = self.object_id(rev)?;
        let commit = self.get_git2_commit(id)?;
        let tree = commit.tree()?;
        let entry = tree.get_path(PathBuf::from(&path).as_ref())?;
        let object = entry.to_object(self.git2_repo())?;
        let blob = object.into_blob().map_err(|_| Error::PathNotFound(path))?;
        Ok(FileContent::new(blob))
    }

    /// Lists branch names with `filter`.
    pub fn branch_names(&self, filter: &Glob<Branch>) -> Result<BranchNames, Error> {
        Ok(self.branches(filter)?.names())
    }

    /// Lists tag names in the local RefScope.
    pub fn tag_names(&self) -> Result<TagNames, Error> {
        Ok(self.tags(&Glob::tags("*")?)?.names())
    }

    /// Returns the Oid of the current HEAD
    pub fn head_oid(&self) -> Result<Oid, Error> {
        let head = self.repo.head()?;
        let head_commit = head.peel_to_commit()?;
        Ok(head_commit.id().into())
    }

    /// Switch to a `namespace`
    pub fn switch_namespace(&self, namespace: &str) -> Result<(), Error> {
        Ok(self.repo.set_namespace(namespace)?)
    }

    /// Returns a full reference name with namespace(s) included.
    pub(crate) fn namespaced_refname<'a>(
        &'a self,
        refname: &Qualified<'a>,
    ) -> Result<Qualified<'a>, Error> {
        let fullname = match self.which_namespace()? {
            Some(namespace) => namespace.to_namespaced(refname).into_qualified(),
            None => refname.clone(),
        };
        Ok(fullname)
    }

    /// Returns a full reference name with namespace(s) included.
    pub(crate) fn namespaced_pattern<'a>(
        &'a self,
        refname: &QualifiedPattern<'a>,
    ) -> Result<QualifiedPattern<'a>, Error> {
        let fullname = match self.which_namespace()? {
            Some(namespace) => namespace.to_namespaced_pattern(refname).into_qualified(),
            None => refname.clone(),
        };
        Ok(fullname)
    }

    /// Get a particular `git2::Commit` of `oid`.
    pub(crate) fn get_git2_commit(&self, oid: Oid) -> Result<git2::Commit, Error> {
        self.repo.find_commit(oid.into()).map_err(Error::Git)
    }

    /// Extract the signature from a commit
    ///
    /// # Arguments
    ///
    /// `commit_oid` - The object ID of the commit
    /// `field` - the name of the header field containing the signature block;
    ///           pass `None` to extract the default 'gpgsig'
    pub fn extract_signature(
        &self,
        commit_oid: &Oid,
        field: Option<&str>,
    ) -> Result<Option<Signature>, Error> {
        // Match is necessary here because according to the documentation for
        // git_commit_extract_signature at
        // https://libgit2.org/libgit2/#HEAD/group/commit/git_commit_extract_signature
        // the return value for a commit without a signature will be GIT_ENOTFOUND
        match self.repo.extract_signature(commit_oid, field) {
            Err(error) => {
                if error.code() == git2::ErrorCode::NotFound {
                    Ok(None)
                } else {
                    Err(error.into())
                }
            },
            Ok(sig) => Ok(Some(Signature::from(sig.0))),
        }
    }

    /// Lists branches that are reachable from `oid`.
    pub fn revision_branches(&self, oid: &Oid, glob: Glob<Branch>) -> Result<Vec<Branch>, Error> {
        let mut contained_branches = vec![];
        for branch in self.branches(&glob)? {
            let branch = branch?;
            let namespaced = self.namespaced_refname(&branch.refname())?;
            let reference = self.repo.find_reference(namespaced.as_str())?;
            if self.reachable_from(&reference, oid)? {
                contained_branches.push(branch);
            }
        }

        Ok(contained_branches)
    }

    fn reachable_from(&self, reference: &git2::Reference, oid: &Oid) -> Result<bool, Error> {
        let git2_oid = (*oid).into();
        let other = reference.peel_to_commit()?.id();
        let is_descendant = self.repo.graph_descendant_of(other, git2_oid)?;

        Ok(other == git2_oid || is_descendant)
    }

    pub(crate) fn diff_commit_and_parents(
        &self,
        path: &file_system::Path,
        commit: &git2::Commit,
    ) -> Result<Option<file_system::Path>, Error> {
        let mut parents = commit.parents();

        let diff = self.diff_commits(Some(path), parents.next().as_ref(), commit)?;
        if let Some(_delta) = diff.deltas().next() {
            Ok(Some(path.clone()))
        } else {
            Ok(None)
        }
    }

    fn diff_commits(
        &self,
        path: Option<&file_system::Path>,
        from: Option<&git2::Commit>,
        to: &git2::Commit,
    ) -> Result<git2::Diff, Error> {
        let new_tree = to.tree()?;
        let old_tree = from.map_or(Ok(None), |c| c.tree().map(Some))?;

        let mut opts = git2::DiffOptions::new();
        if let Some(path) = path {
            opts.pathspec(path);
            // We're skipping the binary pass because we won't be inspecting deltas.
            opts.skip_binary_check(true);
        }

        let mut diff =
            self.repo
                .diff_tree_to_tree(old_tree.as_ref(), Some(&new_tree), Some(&mut opts))?;

        // Detect renames by default.
        let mut find_opts = git2::DiffFindOptions::new();
        find_opts.renames(true);
        diff.find_similar(Some(&mut find_opts))?;

        Ok(diff)
    }

    /// Returns the history with the `head` commit.
    pub fn history<C: ToCommit>(&self, head: C) -> Result<History, Error> {
        History::new(self, head)
    }

    pub(super) fn object_id<R: Revision>(&self, r: &R) -> Result<Oid, Error> {
        r.object_id(self).map_err(|err| Error::Revision(err.into()))
    }

    /// Open a git repository given its exact URI.
    ///
    /// # Errors
    ///
    /// * [`Error::Git`]
    pub fn open(repo_uri: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        let repo = git2::Repository::open(repo_uri)?;
        Ok(Self { repo })
    }

    /// Attempt to open a git repository at or above `repo_uri` in the file
    /// system.
    pub fn discover(repo_uri: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        let repo = git2::Repository::discover(repo_uri)?;
        Ok(Self { repo })
    }

    /// Get a reference to the underlying git2 repo.
    pub(crate) fn git2_repo(&self) -> &git2::Repository {
        &self.repo
    }
}

impl From<git2::Repository> for Repository {
    fn from(repo: git2::Repository) -> Self {
        Repository { repo }
    }
}

impl std::fmt::Debug for Repository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ".git")
    }
}
