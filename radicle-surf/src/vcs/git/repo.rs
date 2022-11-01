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
    file_system::{directory, DirectoryContents, Label},
    vcs::git::{
        error::*,
        reference::{glob::RefGlob, Rev},
        Branch,
        BranchName,
        Commit,
        History,
        Namespace,
        RefScope,
        Revision,
        Signature,
        Stats,
        Tag,
        TagName,
    },
};
use directory::Directory;
use radicle_git_ext::Oid;
use std::{
    collections::{BTreeSet, HashSet},
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
    pub(super) repo_ref: &'a git2::Repository,
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

impl<'a> RepositoryRef<'a> {
    /// What is the current namespace we're browsing in.
    pub fn which_namespace(&self) -> Result<Option<Namespace>, Error> {
        self.repo_ref
            .namespace_bytes()
            .map(Namespace::try_from)
            .transpose()
    }

    /// List the branches within a repository, filtering out ones that do not
    /// parse correctly.
    ///
    /// # Errors
    ///
    /// * [`Error::Git`]
    pub fn list_branches(&self, scope: RefScope) -> Result<Vec<Branch>, Error> {
        RefGlob::branch(scope)
            .references(self)?
            .iter()
            .try_fold(vec![], |mut acc, reference| {
                let branch = Branch::try_from(reference?)?;
                acc.push(branch);
                Ok(acc)
            })
    }

    /// List the tags within a repository, filtering out ones that do not parse
    /// correctly.
    ///
    /// # Errors
    ///
    /// * [`Error::Git`]
    pub fn list_tags(&self, scope: RefScope) -> Result<Vec<Tag>, Error> {
        RefGlob::tag(scope)
            .references(self)?
            .iter()
            .try_fold(vec![], |mut acc, reference| {
                let tag = Tag::try_from(reference?)?;
                acc.push(tag);
                Ok(acc)
            })
    }

    /// List the namespaces within a repository, filtering out ones that do not
    /// parse correctly.
    ///
    /// # Errors
    ///
    /// * [`Error::Git`]
    pub fn list_namespaces(&self) -> Result<Vec<Namespace>, Error> {
        let namespaces: Result<HashSet<Namespace>, Error> = RefGlob::Namespace
            .references(self)?
            .iter()
            .try_fold(HashSet::new(), |mut acc, reference| {
                let namespace = Namespace::try_from(reference?)?;
                acc.insert(namespace);
                Ok(acc)
            });
        Ok(namespaces?.into_iter().collect())
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

    /// Gets a snapshot of the repo as a Directory.
    pub fn snapshot<C: ToCommit>(&self, commit: C) -> Result<Directory, Error> {
        let commit = commit.to_commit(self)?;
        let git2_commit = self.repo_ref.find_commit((commit.id).into())?;
        self.directory_of_commit(&git2_commit)
    }

    /// Returns the last commit, if exists, for a `path` in the history of
    /// `rev`.
    pub fn last_commit(&self, path: file_system::Path, rev: &Rev) -> Result<Option<Commit>, Error> {
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
        let branches = self.list_branches(RefScope::Local)?.len();
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

    /// Lists branch names with `filter`.
    pub fn branch_names(&self, filter: RefScope) -> Result<Vec<BranchName>, Error> {
        let mut branches = self
            .list_branches(filter)?
            .into_iter()
            .map(|b| b.name)
            .collect::<Vec<BranchName>>();

        branches.sort();

        Ok(branches)
    }

    /// Lists tag names in the local RefScope.
    pub fn tag_names(&self) -> Result<Vec<TagName>, Error> {
        let tag_names = self.list_tags(RefScope::Local)?;
        let mut tags: Vec<TagName> = tag_names
            .into_iter()
            .map(|tag_name| tag_name.name())
            .collect();

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
    pub fn revision_branches(&self, oid: &Oid) -> Result<Vec<Branch>, Error> {
        let local = RefGlob::LocalBranch.references(self)?;
        let remote = RefGlob::RemoteBranch { remote: None }.references(self)?;
        let mut references = local.iter().chain(remote.iter());

        let mut contained_branches = vec![];

        references.try_for_each(|reference| {
            let reference = reference?;
            self.reachable_from(&reference, oid).and_then(|contains| {
                if contains {
                    let branch = Branch::try_from(reference)?;
                    contained_branches.push(branch);
                }
                Ok(())
            })
        })?;

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
    ) -> Result<directory::File, Error> {
        let git2_tree = git2_commit.tree()?;
        let entry = git2_tree.get_path(PathBuf::from(&path).as_ref())?;
        let object = entry.to_object(self.repo_ref)?;
        let blob = object.as_blob().ok_or(Error::PathNotFound(path))?;
        Ok(directory::File::new(blob.content()))
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

    /// Generates a Directory for the commit.
    fn directory_of_commit(&self, commit: &git2::Commit) -> Result<Directory, Error> {
        let mut parent_dirs = vec![Directory::root()];
        let tree = commit.as_object().peel_to_tree()?;

        tree.walk(git2::TreeWalkMode::PreOrder, |s, entry| {
            let tree_level = s.split('/').count();
            if tree_level < parent_dirs.len() {
                // As it is PreOrder, the last directory A was visited
                // completely and we are back to the level. Now insert A
                // into its parent directory.
                if let Some(last_dir) = parent_dirs.pop() {
                    if let Some(parent) = parent_dirs.last_mut() {
                        let name = last_dir.name().clone();
                        let content = DirectoryContents::Directory(last_dir);
                        parent.insert(name, content);
                    }
                }
            }

            match entry.kind() {
                Some(git2::ObjectType::Tree) => {
                    if let Some(name) = entry.name() {
                        // Add a new level of directory.
                        match name.parse() {
                            Ok(label) => parent_dirs.push(Directory::new(label)),
                            Err(_) => {
                                return git2::TreeWalkResult::Abort;
                            },
                        }
                    }
                },
                Some(git2::ObjectType::Blob) => {
                    // Construct a File to insert into its parent directory.
                    let object = match entry.to_object(self.repo_ref) {
                        Ok(obj) => obj,
                        Err(_) => {
                            return git2::TreeWalkResult::Abort;
                        },
                    };
                    let blob = match object.as_blob() {
                        Some(b) => b,
                        None => return git2::TreeWalkResult::Abort,
                    };
                    let f = directory::File::new(blob.content());
                    let label = match entry.name() {
                        Some(name) => match name.parse::<Label>() {
                            Ok(label) => label,
                            Err(_) => {
                                return git2::TreeWalkResult::Abort;
                            },
                        },
                        None => return git2::TreeWalkResult::Abort,
                    };
                    let content = DirectoryContents::File {
                        name: label.clone(),
                        file: f,
                    };
                    let parent = match parent_dirs.last_mut() {
                        Some(parent_dir) => parent_dir,
                        None => return git2::TreeWalkResult::Abort,
                    };
                    parent.insert(label, content);
                },
                _ => {
                    return git2::TreeWalkResult::Skip;
                },
            }

            git2::TreeWalkResult::Ok
        })?;

        // Tree walk is complete but there are some levels of dirs
        // that are not popped up from `parent_dirs` yet. Note that
        // the root dir is `parent_dirs[0]`.
        //
        // We need to pop up `parent_dirs` fully and update the directory
        // content at each level.
        while let Some(curr_dir) = parent_dirs.pop() {
            match parent_dirs.last_mut() {
                Some(parent) => {
                    let name = curr_dir.name().clone();
                    let content = DirectoryContents::Directory(curr_dir);
                    parent.insert(name, content);
                },
                None => return Ok(curr_dir), // No more parent, we're at the root.
            }
        }

        Err(Error::RevParseFailure {
            rev: commit.id().to_string(),
        })
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
    /// Open a git repository given its URI.
    ///
    /// # Errors
    ///
    /// * [`Error::Git`]
    pub fn new(repo_uri: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        git2::Repository::open(repo_uri)
            .map(Repository)
            .map_err(Error::from)
    }

    /// Since our operations are read-only when it comes to surfing a repository
    /// we have a separate struct called [`RepositoryRef`]. This turns an owned
    /// [`Repository`], the one returned by [`Repository::new`], into a
    /// [`RepositoryRef`].
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
