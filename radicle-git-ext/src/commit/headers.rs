use std::borrow::Cow;

const BEGIN_SSH: &str = "-----BEGIN SSH SIGNATURE-----\n";
const BEGIN_PGP: &str = "-----BEGIN PGP SIGNATURE-----\n";

/// A collection of headers stored in a [`crate::commit::Commit`].
///
/// Note: these do not include `tree`, `parent`, `author`, and `committer`.
#[derive(Clone, Debug, Default)]
pub struct Headers(pub(super) Vec<(String, String)>);

/// A `gpgsig` signature stored in a [`crate::commit::Commit`].
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

pub struct UnknownScheme;

impl<'a> ToString for Signature<'a> {
    fn to_string(&self) -> String {
        match self {
            Signature::Pgp(pgp) => pgp.to_string(),
            Signature::Ssh(ssh) => ssh.to_string(),
        }
    }
}

impl Headers {
    pub fn new() -> Self {
        Headers(Vec::new())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    pub fn values<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> + '_ {
        self.iter()
            .filter_map(move |(k, v)| (k == name).then_some(v))
    }

    pub fn signatures(&self) -> impl Iterator<Item = Signature> + '_ {
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
