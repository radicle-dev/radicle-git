use std::{marker::PhantomData, path::PathBuf};

use git2::Revwalk;

pub struct History<'a, A> {
    walk: Revwalk<'a>,
    repo: &'a git2::Repository,
    filter_by: Option<FilterBy>,
    marker: PhantomData<A>,
}

enum FilterBy {
    File { name: PathBuf },
}

impl<'a, A> History<'a, A> {
    pub fn new(repo: &'a git2::Repository, start: git2::Oid) -> Result<Self, git2::Error> {
        let mut walk = repo.revwalk()?;
        walk.set_sorting(git2::Sort::TOPOLOGICAL)?;
        walk.simplify_first_parent()?;
        walk.push(start)?;
        Ok(Self {
            walk,
            repo,
            filter_by: None,
            marker: PhantomData,
        })
    }
}

impl<'a> History<'a, BlobAt<'a>> {
    pub fn file(
        repo: &'a git2::Repository,
        start: git2::Oid,
        name: PathBuf,
    ) -> Result<Self, git2::Error> {
        let mut history = Self::new(repo, start)?;
        history.filter_by = Some(FilterBy::File { name });
        Ok(history)
    }
}

impl<'a> Iterator for History<'a, git2::Commit<'a>> {
    type Item = Result<git2::Commit<'a>, git2::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.walk.next() {
            None => None,
            Some(oid) => {
                // TODO: skip if it is not found, perhaps?
                oid.and_then(|oid| self.repo.find_commit(oid))
                    .map(Some)
                    .transpose()
            },
        }
    }
}

pub struct BlobAt<'a> {
    commit: git2::Commit<'a>,
    blob: git2::Blob<'a>,
}

impl<'a> BlobAt<'a> {
    pub fn commit(&self) -> &git2::Commit<'a> {
        &self.commit
    }

    pub fn blob(&self) -> &git2::Blob<'a> {
        &self.blob
    }
}

impl<'a> Iterator for History<'a, BlobAt<'a>> {
    type Item = Result<BlobAt<'a>, git2::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.walk.next() {
            None => None,
            Some(oid) => oid
                .and_then(|oid| {
                    let commit = self.repo.find_commit(oid)?;
                    let tree = commit.tree()?;

                    // debug_tree(&tree)?;

                    match &self.filter_by {
                        Some(FilterBy::File { name }) => {
                            let entry = tree.get_path(name)?;
                            match entry.to_object(self.repo)?.into_blob() {
                                Ok(blob) => Ok(BlobAt { commit, blob }),
                                Err(obj) => Err(git2::Error::new(
                                    git2::ErrorCode::NotFound,
                                    git2::ErrorClass::Object,
                                    &format!(
                                        "history file path filter did not exist, found {}",
                                        obj.kind()
                                            .map(|obj| obj.to_string())
                                            .unwrap_or_else(|| "Unknown Object".to_owned())
                                    ),
                                )),
                            }
                        },
                        None => todo!(),
                    }
                })
                .map(Some)
                .transpose(),
        }
    }
}

fn debug_tree(tree: &git2::Tree) -> Result<(), git2::Error> {
    tree.walk(git2::TreeWalkMode::PreOrder, |s, entry| {
        println!("{}, {:?}", s, entry.name());
        git2::TreeWalkResult::Ok
    })
}
