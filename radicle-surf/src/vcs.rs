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

//! A model of a general VCS. The components consist of a [`History`], a
//! [`Browser`], and a [`Vcs`] trait.

use crate::file_system::directory::Directory;
use nonempty::NonEmpty;

pub mod git;

/// A non-empty bag of artifacts which are used to
/// derive a [`crate::file_system::Directory`] view. Examples of artifacts
/// would be commits in Git or patches in Pijul.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct History<A>(pub NonEmpty<A>);

impl<A> History<A> {
    /// Create a new `History` consisting of one artifact.
    pub fn new(a: A) -> Self {
        History(NonEmpty::new(a))
    }

    /// Push an artifact to the end of the `History`.
    pub fn push(&mut self, a: A) {
        self.0.push(a)
    }

    /// Iterator over the artifacts.
    pub fn iter(&self) -> impl Iterator<Item = &A> {
        self.0.iter()
    }

    /// Get the firest artifact in the `History`.
    pub fn first(&self) -> &A {
        self.0.first()
    }

    /// Get the length of `History` (aka the artefacts count)
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if `History` is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Given that the `History` is topological order from most
    /// recent artifact to least recent, `find_suffix` gets returns
    /// the history up until the point of the given artifact.
    ///
    /// This operation may fail if the artifact does not exist in
    /// the given `History`.
    pub fn find_suffix(&self, artifact: &A) -> Option<Self>
    where
        A: Clone + PartialEq,
    {
        let new_history: Option<NonEmpty<A>> = NonEmpty::from_slice(
            &self
                .iter()
                .cloned()
                .skip_while(|current| *current != *artifact)
                .collect::<Vec<_>>(),
        );

        new_history.map(History)
    }

    /// Apply a function from `A` to `B` over the `History`
    pub fn map<F, B>(self, f: F) -> History<B>
    where
        F: FnMut(A) -> B,
    {
        History(self.0.map(f))
    }

    /// Find an artifact in the `History`.
    ///
    /// The function provided should return `Some` if the item is the desired
    /// output and `None` otherwise.
    pub fn find<F, B>(&self, f: F) -> Option<B>
    where
        F: Fn(&A) -> Option<B>,
    {
        self.iter().find_map(f)
    }

    /// Find an atrifact in the given `History` using the artifacts ID.
    ///
    /// This operation may fail if the artifact does not exist in the history.
    pub fn find_in_history<Identifier, F>(&self, identifier: &Identifier, id_of: F) -> Option<A>
    where
        A: Clone,
        F: Fn(&A) -> Identifier,
        Identifier: PartialEq,
    {
        self.iter()
            .find(|artifact| {
                let current_id = id_of(artifact);
                *identifier == current_id
            })
            .cloned()
    }

    /// Find all occurences of an artifact in a bag of `History`s.
    pub fn find_in_histories<Identifier, F>(
        histories: Vec<Self>,
        identifier: &Identifier,
        id_of: F,
    ) -> Vec<Self>
    where
        A: Clone,
        F: Fn(&A) -> Identifier + Copy,
        Identifier: PartialEq,
    {
        histories
            .into_iter()
            .filter(|history| history.find_in_history(identifier, id_of).is_some())
            .collect()
    }
}

/// A Snapshot is a function that renders a `Directory` given
/// the `Repo` object and a `History` of artifacts.
type Snapshot<A, Repo, Error> = Box<dyn Fn(&Repo, &History<A>) -> Result<Directory, Error>>;

/// A `Browser` is a way of rendering a `History` into a
/// `Directory` snapshot, and the current `History` it is
/// viewing.
pub struct Browser<Repo, A, Error> {
    snapshot: Snapshot<A, Repo, Error>,
    history: History<A>,
    repository: Repo,
}

impl<Repo, A, Error> Browser<Repo, A, Error> {
    /// Get the current `History` the `Browser` is viewing.
    pub fn get(&self) -> History<A>
    where
        A: Clone,
    {
        self.history.clone()
    }

    /// Get the current `History` the `Browser` is viewing as a ref.
    pub fn as_history(&self) -> &History<A> {
        &self.history
    }

    /// Set the `History` the `Browser` should view.
    pub fn set(&mut self, history: History<A>) {
        self.history = history;
    }

    /// Render the `Directory` for this `Browser`.
    pub fn get_directory(&self) -> Result<Directory, Error> {
        (self.snapshot)(&self.repository, &self.history)
    }

    /// Modify the `History` in this `Browser`.
    pub fn modify<F>(&mut self, f: F)
    where
        F: Fn(&History<A>) -> History<A>,
    {
        self.history = f(&self.history)
    }

    /// Change the `Browser`'s view of `History` by modifying it, or
    /// using the default `History` provided if the operation fails.
    pub fn view_at<F>(&mut self, default_history: History<A>, f: F)
    where
        A: Clone,
        F: Fn(&History<A>) -> Option<History<A>>,
    {
        self.modify(|history| f(history).unwrap_or_else(|| default_history.clone()))
    }
}

impl<Repo, A, Error> Vcs<A, Error> for Browser<Repo, A, Error>
where
    Repo: Vcs<A, Error>,
{
    type HistoryId = Repo::HistoryId;
    type ArtefactId = Repo::ArtefactId;

    fn get_history(&self, identifier: Self::HistoryId) -> Result<History<A>, Error> {
        self.repository.get_history(identifier)
    }

    fn get_histories(&self) -> Result<Vec<History<A>>, Error> {
        self.repository.get_histories()
    }

    fn get_identifier(artifact: &A) -> Self::ArtefactId {
        Repo::get_identifier(artifact)
    }
}

pub(crate) trait GetVcs<Error>
where
    Self: Sized,
{
    /// The way to identify a Repository.
    type RepoId;

    /// Find a Repository
    fn get_repo(identifier: Self::RepoId) -> Result<Self, Error>;
}

/// The `VCS` trait encapsulates the minimal amount of information for
/// interacting with some notion of `History` from a given
/// Version-Control-System.
pub trait Vcs<A, Error> {
    /// The way to identify a History.
    type HistoryId;

    /// The way to identify an artefact.
    type ArtefactId;

    /// Find a History in a Repo given a way to identify it
    fn get_history(&self, identifier: Self::HistoryId) -> Result<History<A>, Error>;

    /// Find all histories in a Repo
    fn get_histories(&self) -> Result<Vec<History<A>>, Error>;

    /// Identify artefacts of a Repository
    fn get_identifier(artefact: &A) -> Self::ArtefactId;
}
