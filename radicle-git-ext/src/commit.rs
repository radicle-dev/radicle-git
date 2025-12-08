//! The `git-commit` crate provides parsing a displaying of a [git
//! commit][git-commit].
//!
//! The [`Commit`] data can be constructed using the `FromStr`
//! implementation, or by converting from a `git2::Buf`.
//!
//! The [`Headers`] can be accessed via [`Commit::headers`]. If the
//! signatures of the commit are of particular interest, the
//! [`Commit::signatures`] method can be used, which returns a series of
//! [`Signature`]s.
//!
//! [git-commit]: https://git-scm.com/book/en/v2/Git-Internals-Git-Objects

pub mod headers;
pub mod trailers;

use core::fmt;
use std::str::{self, FromStr};

use git2::{ObjectType, Oid};

use headers::{Headers, Signature};
use trailers::{OwnedTrailer, Trailer, Trailers};

use crate::author::Author;

pub type Commit = CommitData<Oid, Oid>;

impl Commit {
    /// Read the [`Commit`] from the `repo` that is expected to be found at
    /// `oid`.
    pub fn read(repo: &git2::Repository, oid: Oid) -> Result<Self, error::Read> {
        let odb = repo.odb()?;
        let object = odb.read(oid)?;
        Ok(Commit::try_from(object.data())?)
    }

    /// Write the given [`Commit`] to the `repo`. The resulting `Oid`
    /// is the identifier for this commit.
    pub fn write(&self, repo: &git2::Repository) -> Result<Oid, error::Write> {
        let odb = repo.odb().map_err(error::Write::Odb)?;
        self.verify_for_write(&odb)?;
        Ok(odb.write(ObjectType::Commit, self.to_string().as_bytes())?)
    }

    fn verify_for_write(&self, odb: &git2::Odb) -> Result<(), error::Write> {
        for parent in &self.parents {
            verify_object(odb, parent, ObjectType::Commit)?;
        }
        verify_object(odb, &self.tree, ObjectType::Tree)?;

        Ok(())
    }
}

/// A git commit in its object description form, i.e. the output of
/// `git cat-file` for a commit object.
#[derive(Debug)]
pub struct CommitData<Tree, Parent> {
    tree: Tree,
    parents: Vec<Parent>,
    author: Author,
    committer: Author,
    headers: Headers,
    message: String,
    trailers: Vec<OwnedTrailer>,
}

impl<Tree, Parent> CommitData<Tree, Parent> {
    pub fn new<P, I, T>(
        tree: Tree,
        parents: P,
        author: Author,
        committer: Author,
        headers: Headers,
        message: String,
        trailers: I,
    ) -> Self
    where
        P: IntoIterator<Item = Parent>,
        I: IntoIterator<Item = T>,
        OwnedTrailer: From<T>,
    {
        let trailers = trailers.into_iter().map(OwnedTrailer::from).collect();
        let parents = parents.into_iter().collect();
        Self {
            tree,
            parents,
            author,
            committer,
            headers,
            message,
            trailers,
        }
    }

    /// The tree this commit points to.
    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    /// The parents of this commit.
    pub fn parents(&self) -> impl Iterator<Item = Parent> + '_
    where
        Parent: Clone,
    {
        self.parents.iter().cloned()
    }

    /// The author of this commit, i.e. the header corresponding to `author`.
    pub fn author(&self) -> &Author {
        &self.author
    }

    /// The committer of this commit, i.e. the header corresponding to
    /// `committer`.
    pub fn committer(&self) -> &Author {
        &self.committer
    }

    /// The message body of this commit.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The [`Signature`]s found in this commit, i.e. the headers corresponding
    /// to `gpgsig`.
    pub fn signatures<'a>(&'a self) -> impl Iterator<Item = Signature<'a>> + 'a {
        self.headers.signatures()
    }

    /// The [`Headers`] found in this commit.
    ///
    /// Note: these do not include `tree`, `parent`, `author`, and `committer`.
    pub fn headers(&self) -> impl Iterator<Item = (&str, &str)> {
        self.headers.iter()
    }

    /// Iterate over the [`Headers`] values that match the provided `name`.
    pub fn values<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.headers.values(name)
    }

    /// Push a header to the end of the headers section.
    pub fn push_header(&mut self, name: &str, value: &str) {
        self.headers.push(name, value.trim());
    }

    pub fn trailers(&self) -> impl Iterator<Item = &OwnedTrailer> {
        self.trailers.iter()
    }

    /// Convert the `CommitData::tree` into a value of type `U`. The
    /// conversion function `f` can be fallible.
    ///
    /// For example, `map_tree` can be used to turn raw tree data into
    /// an `Oid` by writing it to a repository.
    pub fn map_tree<U, E, F>(self, f: F) -> Result<CommitData<U, Parent>, E>
    where
        F: FnOnce(Tree) -> Result<U, E>,
    {
        Ok(CommitData {
            tree: f(self.tree)?,
            parents: self.parents,
            author: self.author,
            committer: self.committer,
            headers: self.headers,
            message: self.message,
            trailers: self.trailers,
        })
    }

    /// Convert the `CommitData::parents` into a vector containing
    /// values of type `U`. The conversion function `f` can be
    /// fallible.
    ///
    /// For example, `map_parents` can be used to resolve the `Oid`s
    /// to their respective `git2::Commit`s.
    pub fn map_parents<U, E, F>(self, f: F) -> Result<CommitData<Tree, U>, E>
    where
        F: FnMut(Parent) -> Result<U, E>,
    {
        Ok(CommitData {
            tree: self.tree,
            parents: self
                .parents
                .into_iter()
                .map(f)
                .collect::<Result<Vec<_>, _>>()?,
            author: self.author,
            committer: self.committer,
            headers: self.headers,
            message: self.message,
            trailers: self.trailers,
        })
    }
}

fn verify_object(odb: &git2::Odb, oid: &Oid, expected: ObjectType) -> Result<(), error::Write> {
    use git2::{Error, ErrorClass, ErrorCode};

    let (_, kind) = odb
        .read_header(*oid)
        .map_err(|err| error::Write::OdbRead { oid: *oid, err })?;
    if kind != expected {
        Err(error::Write::NotCommit {
            oid: *oid,
            err: Error::new(
                ErrorCode::NotFound,
                ErrorClass::Object,
                format!("Object '{oid}' is not expected object type {expected}"),
            ),
        })
    } else {
        Ok(())
    }
}

pub mod error {
    use std::str;

    use thiserror::Error;

    use crate::author;

    #[derive(Debug, Error)]
    pub enum Write {
        #[error(transparent)]
        Git(#[from] git2::Error),
        #[error("the parent '{oid}' provided is not a commit object")]
        NotCommit {
            oid: git2::Oid,
            #[source]
            err: git2::Error,
        },
        #[error("failed to access git odb")]
        Odb(#[source] git2::Error),
        #[error("failed to read '{oid}' from git odb")]
        OdbRead {
            oid: git2::Oid,
            #[source]
            err: git2::Error,
        },
    }

    #[derive(Debug, Error)]
    pub enum Read {
        #[error(transparent)]
        Git(#[from] git2::Error),
        #[error(transparent)]
        Parse(#[from] Parse),
    }

    #[derive(Debug, Error)]
    pub enum Parse {
        #[error(transparent)]
        Author(#[from] author::ParseError),
        #[error("invalid '{header}'")]
        InvalidHeader {
            header: &'static str,
            #[source]
            err: git2::Error,
        },
        #[error("invalid git commit object format")]
        InvalidFormat,
        #[error("missing '{0}' while parsing commit")]
        Missing(&'static str),
        #[error("error occurred while checking for git-trailers: {0}")]
        Trailers(#[source] git2::Error),
        #[error(transparent)]
        Utf8(#[from] str::Utf8Error),
    }
}

impl TryFrom<git2::Buf> for Commit {
    type Error = error::Parse;

    fn try_from(value: git2::Buf) -> Result<Self, Self::Error> {
        value.as_str().ok_or(error::Parse::InvalidFormat)?.parse()
    }
}

impl TryFrom<&[u8]> for Commit {
    type Error = error::Parse;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Commit::from_str(str::from_utf8(data)?)
    }
}

impl FromStr for Commit {
    type Err = error::Parse;

    fn from_str(buffer: &str) -> Result<Self, Self::Err> {
        let (header, message) = buffer
            .split_once("\n\n")
            .ok_or(error::Parse::InvalidFormat)?;
        let mut lines = header.lines();

        let tree = match lines.next() {
            Some(tree) => tree
                .strip_prefix("tree ")
                .map(git2::Oid::from_str)
                .transpose()
                .map_err(|err| error::Parse::InvalidHeader {
                    header: "tree",
                    err,
                })?
                .ok_or(error::Parse::Missing("tree"))?,
            None => return Err(error::Parse::Missing("tree")),
        };

        let mut parents = Vec::new();
        let mut author: Option<Author> = None;
        let mut committer: Option<Author> = None;
        let mut headers = Headers::new();

        for line in lines {
            // Check if a signature is still being parsed
            if let Some(rest) = line.strip_prefix(' ') {
                let value: &mut String = headers
                    .0
                    .last_mut()
                    .map(|(_, v)| v)
                    .ok_or(error::Parse::InvalidFormat)?;
                value.push('\n');
                value.push_str(rest);
                continue;
            }

            if let Some((name, value)) = line.split_once(' ') {
                match name {
                    "parent" => parents.push(git2::Oid::from_str(value).map_err(|err| {
                        error::Parse::InvalidHeader {
                            header: "parent",
                            err,
                        }
                    })?),
                    "author" => author = Some(value.parse::<Author>()?),
                    "committer" => committer = Some(value.parse::<Author>()?),
                    _ => headers.push(name, value),
                }
                continue;
            }
        }

        let trailers = Trailers::parse(message).map_err(error::Parse::Trailers)?;

        let message = message
            .strip_suffix(&trailers.to_string(": "))
            .unwrap_or(message)
            .to_string();

        let trailers = trailers.iter().map(OwnedTrailer::from).collect();

        Ok(Self {
            tree,
            parents,
            author: author.ok_or(error::Parse::Missing("author"))?,
            committer: committer.ok_or(error::Parse::Missing("committer"))?,
            headers,
            message,
            trailers,
        })
    }
}

impl fmt::Display for Commit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "tree {}", self.tree)?;
        for parent in self.parents() {
            writeln!(f, "parent {parent}")?;
        }
        writeln!(f, "author {}", self.author)?;
        writeln!(f, "committer {}", self.committer)?;

        for (name, value) in self.headers.iter() {
            writeln!(f, "{name} {}", value.replace('\n', "\n "))?;
        }
        writeln!(f)?;
        write!(f, "{}", self.message.trim())?;
        writeln!(f)?;

        if !self.trailers.is_empty() {
            writeln!(f)?;
        }
        for trailer in self.trailers.iter() {
            writeln!(f, "{}", Trailer::from(trailer).display(": "))?;
        }
        Ok(())
    }
}
