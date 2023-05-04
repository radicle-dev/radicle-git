use std::{borrow::Cow, fmt, fmt::Write, ops::Deref, str::FromStr};

use git2::{MessageTrailersStrs, MessageTrailersStrsIterator};

/// A Git commit's set of trailers that are left in the commit's
/// message.
///
/// Trailers are key/value pairs in the last paragraph of a message,
/// not including any patches or conflicts that may be present.
///
/// # Usage
///
/// To construct `Trailers`, you can use [`Trailers::parse`] or its
/// `FromStr` implementation.
///
/// To iterate over the trailers, you can use [`Trailers::iter`].
///
/// To render the trailers to a `String`, you can use
/// [`Trailers::to_string`] or its `Display` implementation (note that
/// it will default to using `": "` as the separator.
///
/// # Examples
///
/// ```text
/// Add new functionality
///
/// Making code better with new functionality.
///
/// X-Signed-Off-By: Alex Sellier
/// X-Co-Authored-By: Fintan Halpenny
/// ```
///
/// The trailers in the above example are:
///
/// ```text
/// X-Signed-Off-By: Alex Sellier
/// X-Co-Authored-By: Fintan Halpenny
/// ```
pub struct Trailers {
    inner: MessageTrailersStrs,
}

impl Trailers {
    pub fn parse(message: &str) -> Result<Self, git2::Error> {
        Ok(Self {
            inner: git2::message_trailers_strs(message)?,
        })
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            inner: self.inner.iter(),
        }
    }

    pub fn to_string<'a, S>(&self, sep: S) -> String
    where
        S: Separator<'a>,
    {
        let mut buf = String::new();
        for (i, trailer) in self.iter().enumerate() {
            if i > 0 {
                writeln!(buf).ok();
            }

            write!(buf, "{}", trailer.display(sep.sep_for(&trailer.token))).ok();
        }
        writeln!(buf).ok();
        buf
    }
}

impl fmt::Display for Trailers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string(": "))
    }
}

pub trait Separator<'a> {
    fn sep_for(&self, token: &Token) -> &'a str;
}

impl<'a> Separator<'a> for &'a str {
    fn sep_for(&self, _: &Token) -> &'a str {
        self
    }
}

impl<'a, F> Separator<'a> for F
where
    F: Fn(&Token) -> &'a str,
{
    fn sep_for(&self, token: &Token) -> &'a str {
        self(token)
    }
}

impl FromStr for Trailers {
    type Err = git2::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

pub struct Iter<'a> {
    inner: MessageTrailersStrsIterator<'a>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Trailer<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (token, value) = self.inner.next()?;
        Some(Trailer {
            token: Token(token),
            value: Cow::Borrowed(value),
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Token<'a>(&'a str);

impl Deref for Token<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a> TryFrom<&'a str> for Token<'a> {
    type Error = &'static str;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        let is_token = s.chars().all(|c| c.is_alphanumeric() || c == '-');
        if is_token {
            Ok(Token(s))
        } else {
            Err("token contains invalid characters")
        }
    }
}

pub struct Display<'a> {
    trailer: &'a Trailer<'a>,
    separator: &'a str,
}

impl<'a> fmt::Display for Display<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}{}{}",
            self.trailer.token.deref(),
            self.separator,
            self.trailer.value,
        )
    }
}

/// A trailer is a key/value pair found in the last paragraph of a Git
/// commit message, not including any patches or conflicts that may be
/// present.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Trailer<'a> {
    pub token: Token<'a>,
    pub value: Cow<'a, str>,
}

impl<'a> Trailer<'a> {
    pub fn display(&'a self, separator: &'a str) -> Display<'a> {
        Display {
            trailer: self,
            separator,
        }
    }

    pub fn to_owned(&self) -> OwnedTrailer {
        OwnedTrailer::from(self)
    }
}

/// A version of the [`Trailer`] which owns its token and
/// value. Useful for when you need to carry trailers around in a long
/// lived data structure.
#[derive(Debug)]
pub struct OwnedTrailer {
    pub token: OwnedToken,
    pub value: String,
}

#[derive(Debug)]
pub struct OwnedToken(String);

impl Deref for OwnedToken {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<&Trailer<'a>> for OwnedTrailer {
    fn from(t: &Trailer<'a>) -> Self {
        OwnedTrailer {
            token: OwnedToken(t.token.0.to_string()),
            value: t.value.to_string(),
        }
    }
}

impl<'a> From<Trailer<'a>> for OwnedTrailer {
    fn from(t: Trailer<'a>) -> Self {
        (&t).into()
    }
}

impl<'a> From<&'a OwnedTrailer> for Trailer<'a> {
    fn from(t: &'a OwnedTrailer) -> Self {
        Trailer {
            token: Token(t.token.0.as_str()),
            value: Cow::from(&t.value),
        }
    }
}
