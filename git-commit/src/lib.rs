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

use std::{
    fmt::Write as _,
    str::{self, FromStr},
};

use git2::{ObjectType, Oid};
use git_trailers::{self as trailers, OwnedTrailer, Trailer};

pub mod author;
pub use author::Author;

pub mod headers;
pub use headers::{Headers, Signature};

/// A git commit in its object description form, i.e. the output of
/// `git cat-file` for a commit object.
#[derive(Debug)]
pub struct Commit {
    tree: Oid,
    parents: Vec<Oid>,
    author: Author,
    committer: Author,
    headers: Headers,
    message: String,
    trailers: Vec<OwnedTrailer>,
}

impl Commit {
    pub fn new<I, T>(
        tree: Oid,
        parents: Vec<Oid>,
        author: Author,
        committer: Author,
        headers: Headers,
        message: String,
        trailers: I,
    ) -> Self
    where
        I: IntoIterator<Item = T>,
        OwnedTrailer: From<T>,
    {
        let trailers = trailers.into_iter().map(OwnedTrailer::from).collect();
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

    /// Read the [`Commit`] from the `repo` that is expected to be found at
    /// `oid`.
    pub fn read(repo: &git2::Repository, oid: Oid) -> Result<Self, error::Read> {
        let odb = repo.odb()?;
        let object = odb.read(oid)?;
        Ok(Commit::try_from(object.data())?)
    }

    /// Write the given [`Commit`] to the `repo`. The resulting `Oid`
    /// is the identifier for this commit.
    pub fn write(&self, repo: &git2::Repository) -> Result<Oid, git2::Error> {
        let odb = repo.odb()?;
        odb.write(ObjectType::Commit, self.to_string().as_bytes())
    }

    /// The tree [`Oid`] this commit points to.
    pub fn tree(&self) -> Oid {
        self.tree
    }

    /// The parent [`Oid`]s of this commit.
    pub fn parents(&self) -> impl Iterator<Item = Oid> + '_ {
        self.parents.iter().copied()
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
    pub fn signatures(&self) -> impl Iterator<Item = Signature> + '_ {
        self.headers.signatures()
    }

    /// The [`Headers`] found in this commit.
    ///
    /// Note: these do not include `tree`, `parent`, `author`, and `committer`.
    pub fn headers(&self) -> impl Iterator<Item = (&str, &str)> {
        self.headers.iter()
    }

    /// Iterate over the [`Headers`] values that match the provided `name`.
    pub fn values<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> + '_ {
        self.headers.values(name)
    }

    /// Push a header to the end of the headers section.
    pub fn push_header(&mut self, name: &str, value: &str) {
        self.headers.push(name, value.trim());
    }

    pub fn trailers(&self) -> impl Iterator<Item = &OwnedTrailer> {
        self.trailers.iter()
    }
}

pub mod error {
    use std::str;

    use thiserror::Error;

    use super::author;

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
        #[error(transparent)]
        Token(#[from] git_trailers::InvalidToken),
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

        let (message, trailers) = message.lines().fold(
            (Vec::new(), Vec::new()),
            |(mut message, mut trailers), line| match trailers::parser::trailer(line, ": ") {
                Ok((_, trailer)) => {
                    trailers.push(trailer.into());
                    (message, trailers)
                },
                Err(_) => {
                    message.push(line);
                    (message, trailers)
                },
            },
        );

        Ok(Self {
            tree,
            parents,
            author: author.ok_or(error::Parse::Missing("author"))?,
            committer: committer.ok_or(error::Parse::Missing("committer"))?,
            headers,
            message: message.join("\n"),
            trailers,
        })
    }
}

impl ToString for Commit {
    fn to_string(&self) -> String {
        let mut buf = String::new();

        writeln!(buf, "tree {}", self.tree).ok();

        for parent in &self.parents {
            writeln!(buf, "parent {}", parent).ok();
        }

        writeln!(buf, "author {}", self.author).ok();
        writeln!(buf, "committer {}", self.committer).ok();

        for (name, value) in self.headers.iter() {
            writeln!(buf, "{} {}", name, value.replace('\n', "\n ")).ok();
        }
        writeln!(buf).ok();
        write!(buf, "{}", self.message).ok();

        if !self.trailers.is_empty() {
            writeln!(buf).ok();
        }
        for (i, trailer) in self.trailers.iter().enumerate() {
            if i < self.trailers.len() {
                writeln!(buf, "{}", Trailer::from(trailer).display(": ")).ok();
            } else {
                write!(buf, "{}", Trailer::from(trailer).display(": ")).ok();
            }
        }
        buf
    }
}
