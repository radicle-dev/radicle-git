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

use crate::{
    diff::*,
    file_system,
    file_system::{directory, DirectoryEntry, Label},
    git::{
        error::*,
        Branch,
        BranchName,
        Commit,
        Glob,
        History,
        Namespace,
        Revision,
        Signature,
        Stats,
        Tag,
        TagName,
    },
};
use directory::{Directory, FileContent};
use radicle_git_ext::Oid;
use std::{
    collections::{btree_set, BTreeMap, BTreeSet},
    convert::TryFrom,
    path::PathBuf,
    str,
};

use super::commit::ToCommit;

/// Wrapper around the `git2`'s `git2::Repository` type.
/// This is to to limit the functionality that we can do
/// on the underlying object.
pub struct Repository(pub(super) git2::Repository);

/// A reference-only `Repository`. This means that we cannot mutate the
/// underlying `Repository`. Not being able to mutate the `Repository` means
/// that the functions defined for `RepositoryRef` should be thread-safe.
///
/// # Construction
///
/// Use the `From<&'a git2::Repository>` implementation to construct a
/// `RepositoryRef`.
#[derive(Clone, Copy)]
pub struct RepositoryRef<'a> {
    pub(crate) repo_ref: &'a git2::Repository,
}

// RepositoryRef should be safe to transfer across thread boundaries since it
// only holds a reference to git2::Repository. git2::Repository is also Send
// (see: https://docs.rs/git2/0.13.5/src/git2/repo.rs.html#46)
unsafe impl<'a> Send for RepositoryRef<'a> {}

impl<'a> From<&'a git2::Repository> for RepositoryRef<'a> {
    fn from(repo_ref: &'a git2::Repository) -> Self {
        RepositoryRef { repo_ref }
    }
}

// I think the following `Tags` and `Branches` would be merged
// using Generic associated types supported in Rust 1.65.0.

/// An iterator for tags.
pub struct Tags<'a> {
    references: Vec<git2::References<'a>>,
    current: usize,
}

impl<'a> Iterator for Tags<'a> {
    type Item = Result<Tag, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.current < self.references.len() {
            match self.references.get_mut(self.current) {
                Some(refs) => match refs.next() {
                    Some(res) => return Some(res.map_err(Error::Git).and_then(Tag::try_from)),
                    None => self.current += 1,
                },
                None => break,
            }
        }
        None
    }
}

/// An iterator for branches.
pub struct Branches<'a> {
    references: Vec<git2::References<'a>>,
    current: usize,
}

impl<'a> Iterator for Branches<'a> {
    type Item = Result<Branch, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.current < self.references.len() {
            match self.references.get_mut(self.current) {
                Some(refs) => match refs.next() {
                    Some(res) => return Some(res.map_err(Error::Git).and_then(Branch::try_from)),
                    None => self.current += 1,
                },
                None => break,
            }
        }
        None
    }
}

/// An iterator for namespaces.
pub struct Namespaces {
    namespaces: btree_set::IntoIter<Namespace>,
}

impl Iterator for Namespaces {
    type Item = Namespace;
    fn next(&mut self) -> Option<Self::Item> {
        self.namespaces.next()
    }
}

impl<'a> RepositoryRef<'a> {
    /// What is the current namespace we're browsing in.
    pub fn which_namespace(&self) -> Result<Option<Namespace>, Error> {
        self.repo_ref
            .namespace_bytes()
            .map(Namespace::try_from)
            .transpose()
    }

    /// Returns an iterator of branches that match `pattern`.
    pub fn branches(&self, pattern: &Glob<Branch>) -> Result<Branches, Error> {
        let mut branches = Branches {
            references: vec![],
            current: 0,
        };
        for glob in pattern.globs().iter() {
            let namespaced = self.namespaced_refname(glob)?;
            let references = self.repo_ref.references_glob(&namespaced)?;
            branches.references.push(references);
        }
        Ok(branches)
    }

    /// Returns an iterator of tags that match `pattern`.
    pub fn tags(&self, pattern: &Glob<Tag>) -> Result<Tags, Error> {
        let mut tags = Tags {
            references: vec![],
            current: 0,
        };
        for glob in pattern.globs().iter() {
            let namespaced = self.namespaced_refname(glob)?;
            let references = self.repo_ref.references_glob(&namespaced)?;
            tags.references.push(references);
        }
        Ok(tags)
    }

    /// Returns an iterator of namespaces that match `pattern`.
    pub fn namespaces(&self, pattern: &Glob<Namespace>) -> Result<Namespaces, Error> {
        let mut set = BTreeSet::new();
        for glob in pattern.globs().iter() {
            let new_set = self
                .repo_ref
                .references_glob(glob)?
                .map(|reference| {
                    reference
                        .map_err(Error::Git)
                        .and_then(|r| Namespace::try_from(r).map_err(|_| Error::EmptyNamespace))
                })
                .collect::<Result<BTreeSet<Namespace>, Error>>()?;
            set.extend(new_set);
        }
        Ok(Namespaces {
            namespaces: set.into_iter(),
        })
    }

    /// Get the [`Diff`] between two commits.
    pub fn diff(&self, from: impl Revision, to: impl Revision) -> Result<Diff, Error> {
        let from_commit = self.get_git2_commit(from.object_id(self)?)?;
        let to_commit = self.get_git2_commit(to.object_id(self)?)?;
        self.diff_commits(None, Some(&from_commit), &to_commit)
            .and_then(|diff| Diff::try_from(diff).map_err(Error::from))
    }

    /// Get the [`Diff`] of a commit with no parents.
    pub fn initial_diff<R: Revision>(&self, rev: R) -> Result<Diff, Error> {
        let commit = self.get_git2_commit(rev.object_id(self)?)?;
        self.diff_commits(None, None, &commit)
            .and_then(|diff| Diff::try_from(diff).map_err(Error::from))
    }

    /// Get the diff introduced by a particlar rev.
    pub fn diff_from_parent<C: ToCommit>(&self, commit: C) -> Result<Diff, Error> {
        let commit = commit.to_commit(self)?;
        match commit.parents.first() {
            Some(parent) => self.diff(*parent, commit.id),
            None => self.initial_diff(commit.id),
        }
    }

    /// Parse an [`Oid`] from the given string.
    pub fn oid(&self, oid: &str) -> Result<Oid, Error> {
        Ok(self.repo_ref.revparse_single(oid)?.id().into())
    }

    /// Returns a top level `Directory` without nested sub-directories.
    ///
    /// To visit inside any nested sub-directories, call `directory.get(&repo)`
    /// on the sub-directory.
    pub fn root_dir<C: ToCommit>(&self, commit: C) -> Result<Directory, Error> {
        let commit = commit.to_commit(self)?;
        let git2_commit = self.repo_ref.find_commit((commit.id).into())?;
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
        let git2_tree = self.repo_ref.find_tree(d.oid.into())?;
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
        let oid = rev.object_id(self)?;
        match self.repo_ref.find_commit(oid.into()) {
            Ok(commit) => Commit::try_from(commit),
            Err(e) => Err(Error::Git(e)),
        }
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
        let blob = self.repo_ref.find_blob(object_id.into())?;
        Ok(FileContent::new(blob))
    }

    /// Return the size of a file
    pub(crate) fn file_size(&self, oid: Oid) -> Result<usize, Error> {
        let blob = self.repo_ref.find_blob(oid.into())?;
        Ok(blob.size())
    }

    /// Lists branch names with `filter`.
    pub fn branch_names(&self, filter: &Glob<Branch>) -> Result<Vec<BranchName>, Error> {
        let branches: Result<Vec<BranchName>, Error> =
            self.branches(filter)?.map(|b| b.map(|b| b.name)).collect();
        let mut branches = branches?;
        branches.sort();

        Ok(branches)
    }

    /// Lists tag names in the local RefScope.
    pub fn tag_names(&self) -> Result<Vec<TagName>, Error> {
        let mut tags = self
            .tags(&Glob::tags("*")?)?
            .map(|t| t.map(|t| t.name()))
            .collect::<Result<Vec<TagName>, Error>>()?;
        tags.sort();

        Ok(tags)
    }

    /// Returns the Oid of the current HEAD
    pub fn head_oid(&self) -> Result<Oid, Error> {
        let head = self.repo_ref.head()?;
        let head_commit = head.peel_to_commit()?;
        Ok(head_commit.id().into())
    }

    /// Switch to a `namespace`
    pub fn switch_namespace(&self, namespace: &str) -> Result<(), Error> {
        Ok(self.repo_ref.set_namespace(namespace)?)
    }

    /// Returns a full reference name with namespace(s) included.
    pub(crate) fn namespaced_refname(&self, refname: &str) -> Result<String, Error> {
        let fullname = match self.which_namespace()? {
            Some(namespace) => namespace.append_refname(refname),
            None => refname.to_string(),
        };
        Ok(fullname)
    }

    pub(crate) fn refname_to_oid(&self, refname: &str) -> Result<Oid, Error> {
        let oid = self.repo_ref.refname_to_id(refname)?;
        Ok(oid.into())
    }

    /// Get a particular `git2::Commit` of `oid`.
    pub(crate) fn get_git2_commit(&self, oid: Oid) -> Result<git2::Commit, Error> {
        self.repo_ref.find_commit(oid.into()).map_err(Error::Git)
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
        match self.repo_ref.extract_signature(commit_oid, field) {
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
    pub fn revision_branches(&self, oid: &Oid, glob: &Glob<Branch>) -> Result<Vec<Branch>, Error> {
        let mut contained_branches = vec![];
        for branch in self.branches(glob)? {
            let branch = branch?;
            let namespaced = self.namespaced_refname(&branch.refname())?;
            let reference = self.repo_ref.find_reference(&namespaced)?;
            if self.reachable_from(&reference, oid)? {
                contained_branches.push(branch);
            }
        }

        Ok(contained_branches)
    }

    fn reachable_from(&self, reference: &git2::Reference, oid: &Oid) -> Result<bool, Error> {
        let git2_oid = (*oid).into();
        let other = reference.peel_to_commit()?.id();
        let is_descendant = self.repo_ref.graph_descendant_of(other, git2_oid)?;

        Ok(other == git2_oid || is_descendant)
    }

    pub(crate) fn get_commit_file(
        &self,
        git2_commit: &git2::Commit,
        path: file_system::Path,
    ) -> Result<FileContent, Error> {
        let git2_tree = git2_commit.tree()?;
        let entry = git2_tree.get_path(PathBuf::from(&path).as_ref())?;
        let object = entry.to_object(self.repo_ref)?;
        let blob = object.into_blob().map_err(|_| Error::PathNotFound(path))?;
        Ok(FileContent::new(blob))
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
            self.repo_ref
                .diff_tree_to_tree(old_tree.as_ref(), Some(&new_tree), Some(&mut opts))?;

        // Detect renames by default.
        let mut find_opts = git2::DiffFindOptions::new();
        find_opts.renames(true);
        diff.find_similar(Some(&mut find_opts))?;

        Ok(diff)
    }

    /// Returns the history with the `head` commit.
    pub fn history<C: ToCommit>(&self, head: C) -> Result<History, Error> {
        History::new(*self, head)
    }
}

impl<'a> std::fmt::Debug for RepositoryRef<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ".git")
    }
}

impl Repository {
    /// Open a git repository given its exact URI.
    ///
    /// # Errors
    ///
    /// * [`Error::Git`]
    pub fn open(repo_uri: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        git2::Repository::open(repo_uri)
            .map(Repository)
            .map_err(Error::from)
    }

    /// Attempt to open a git repository at or above `repo_uri` in the file
    /// system.
    pub fn discover(repo_uri: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        git2::Repository::discover(repo_uri)
            .map(Repository)
            .map_err(Error::from)
    }

    /// Since our operations are read-only when it comes to surfing a repository
    /// we have a separate struct called [`RepositoryRef`]. This turns an owned
    /// [`Repository`] into a [`RepositoryRef`].
    pub fn as_ref(&'_ self) -> RepositoryRef<'_> {
        RepositoryRef { repo_ref: &self.0 }
    }
}

impl<'a> From<&'a Repository> for RepositoryRef<'a> {
    fn from(repo: &'a Repository) -> Self {
        repo.as_ref()
    }
}

impl From<git2::Repository> for Repository {
    fn from(repo: git2::Repository) -> Self {
        Repository(repo)
    }
}

impl std::fmt::Debug for Repository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ".git")
    }
}
