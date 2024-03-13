use std::{convert::Infallible, io, path::Path};

use git2::Oid;
use radicle_git_ext::{commit::CommitData, ref_format::RefString};
use test_helpers::tempdir::{self, WithTmpDir};

use crate::gen::commit::{self, TreeData};

pub struct Fixture {
    pub inner: WithTmpDir<git2::Repository>,
    pub head: Option<git2::Oid>,
}

/// Initialise a [`git2::Repository`] in a temporary directory.
///
/// The provided `commits` will be added to the repository, and the
/// head commit will be returned.
pub fn fixture(
    refname: &RefString,
    commits: Vec<CommitData<TreeData, Infallible>>,
) -> io::Result<Fixture> {
    let repo = tempdir::WithTmpDir::new(|path| git2::Repository::init(path).map_err(io_other))?;
    let commits = commit::write_commits(&repo, commits).map_err(io_other)?;
    let head = commits.last().copied();

    if let Some(head) = head {
        repo.reference(refname.as_str(), head, false, "Initialise repository")
            .map_err(io_other)?;
    }

    Ok(Fixture { inner: repo, head })
}

pub fn bare_fixture(
    refname: &RefString,
    commits: Vec<CommitData<TreeData, Infallible>>,
) -> io::Result<Fixture> {
    let repo =
        tempdir::WithTmpDir::new(|path| git2::Repository::init_bare(path).map_err(io_other))?;
    let commits = commit::write_commits(&repo, commits).map_err(io_other)?;
    let head = commits.last().copied();

    if let Some(head) = head {
        repo.reference(refname.as_str(), head, false, "Initialise repository")
            .map_err(io_other)?;
    }

    Ok(Fixture { inner: repo, head })
}

pub fn submodule<'a>(
    parent: &'a git2::Repository,
    child: &'a git2::Repository,
    refname: &RefString,
    head: Oid,
    author: &git2::Signature,
) -> io::Result<git2::Submodule<'a>> {
    let url = format!("file://{}", child.path().canonicalize()?.display());
    let mut sub = parent
        .submodule(url.as_str(), Path::new("submodule"), true)
        .map_err(io_other)?;
    sub.open().map_err(io_other)?;
    sub.clone(Some(&mut git2::SubmoduleUpdateOptions::default()))
        .map_err(io_other)?;
    sub.add_to_index(true).map_err(io_other)?;
    sub.add_finalize().map_err(io_other)?;
    {
        let mut ix = parent.index().map_err(io_other)?;
        let tree = ix.write_tree_to(parent).map_err(io_other)?;
        let tree = parent.find_tree(tree).map_err(io_other)?;
        let head = parent.find_commit(head).map_err(io_other)?;
        parent
            .commit(
                Some(refname.as_str()),
                author,
                author,
                "Commit submodule",
                &tree,
                &[&head],
            )
            .map_err(io_other)?;
    }
    Ok(sub)
}

fn io_other<E>(e: E) -> io::Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    io::Error::new(io::ErrorKind::Other, e)
}
