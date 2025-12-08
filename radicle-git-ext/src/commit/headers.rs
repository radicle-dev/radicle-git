use core::fmt;
use std::borrow::Cow;

const BEGIN_SSH: &str = "-----BEGIN SSH SIGNATURE-----\n";
const BEGIN_PGP: &str = "-----BEGIN PGP SIGNATURE-----\n";

/// A collection of headers stored in a [`crate::commit::Commit`].
///
/// Note: these do not include `tree`, `parent`, `author`, and `committer`.
#[derive(Clone, Debug, Default)]
pub struct Headers(pub(super) Vec<(String, String)>);

/// A `gpgsig` signature stored in a [`crate::commit::Commit`].
#[derive(Debug)]
pub enum Signature<'a> {
    /// A PGP signature, i.e. starts with `-----BEGIN PGP SIGNATURE-----`.
    Pgp(Cow<'a, str>),
    /// A SSH signature, i.e. starts with `-----BEGIN SSH SIGNATURE-----`.
    Ssh(Cow<'a, str>),
}

impl<'a> Signature<'a> {
    fn from_str(s: &'a str) -> Result<Self, UnknownScheme> {
        if s.starts_with(BEGIN_SSH) {
            Ok(Signature::Ssh(Cow::Borrowed(s)))
        } else if s.starts_with(BEGIN_PGP) {
            Ok(Signature::Pgp(Cow::Borrowed(s)))
        } else {
            Err(UnknownScheme)
        }
    }
}

impl fmt::Display for Signature<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Signature::Pgp(pgp) => f.write_str(pgp.as_ref()),
            Signature::Ssh(ssh) => f.write_str(ssh.as_ref()),
        }
    }
}

pub struct UnknownScheme;

impl Headers {
    pub fn new() -> Self {
        Headers(Vec::new())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    pub fn values<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.iter()
            .filter_map(move |(k, v)| (k == name).then_some(v))
    }

    pub fn signatures<'a>(&'a self) -> impl Iterator<Item = Signature<'a>> + 'a {
        self.0.iter().filter_map(|(k, v)| {
            if k == "gpgsig" {
                Signature::from_str(v).ok()
            } else {
                None
            }
        })
    }

    /// Push a header to the end of the headers section.
    pub fn push(&mut self, name: &str, value: &str) {
        self.0.push((name.to_owned(), value.trim().to_owned()));
    }
}
