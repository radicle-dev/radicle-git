use std::{
    fmt,
    num::ParseIntError,
    str::{self, FromStr},
};

use thiserror::Error;

/// The data for indicating authorship of an action within a
/// [`super::Commit`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Author {
    /// Name corresponding to `user.name` in the git config.
    ///
    /// Note: this must not contain `<` or `>`.
    pub name: String,
    /// Email corresponding to `user.email` in the git config.
    ///
    /// Note: this must not contain `<` or `>`.
    pub email: String,
    /// The time of this author's action.
    pub time: Time,
}

/// The time of a [`Author`]'s action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Time {
    seconds: i64,
    offset: i32,
}

impl Time {
    pub fn new(seconds: i64, offset: i32) -> Self {
        Self { seconds, offset }
    }

    /// Return the time, in seconds, since the epoch.
    pub fn seconds(&self) -> i64 {
        self.seconds
    }

    /// Return the timezone offset, in minutes.
    pub fn offset(&self) -> i32 {
        self.offset
    }
}

impl From<Time> for git2::Time {
    fn from(t: Time) -> Self {
        Self::new(t.seconds, t.offset)
    }
}

impl From<git2::Time> for Time {
    fn from(t: git2::Time) -> Self {
        Self::new(t.seconds(), t.offset_minutes())
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.offset.is_negative() { '-' } else { '+' };
        write!(f, "{} {}{:0>4}", self.seconds, sign, self.offset.abs())
    }
}

impl fmt::Display for Author {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} <{}> {}", self.name, self.email, self.time,)
    }
}

impl TryFrom<&Author> for git2::Signature<'_> {
    type Error = git2::Error;

    fn try_from(person: &Author) -> Result<Self, Self::Error> {
        let time = git2::Time::new(person.time.seconds, person.time.offset);
        git2::Signature::new(&person.name, &person.email, &time)
    }
}

impl<'a> TryFrom<&git2::Signature<'a>> for Author {
    type Error = str::Utf8Error;

    fn try_from(value: &git2::Signature<'a>) -> Result<Self, Self::Error> {
        Ok(Self {
            name: str::from_utf8(value.name_bytes())?.to_string(),
            email: str::from_utf8(value.email_bytes())?.to_string(),
            time: value.when().into(),
        })
    }
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("missing '{0}' while parsing person signature")]
    Missing(&'static str),
    #[error("offset was incorrect format while parsing person signature")]
    Offset(#[source] ParseIntError),
    #[error("time was incorrect format while parsing person signature")]
    Time(#[source] ParseIntError),
    #[error("time offset is expected to be '+'/'-' for a person siganture")]
    UnknownOffset,
}

impl FromStr for Author {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut components = s.split(' ');
        let offset = match components.next_back() {
            None => return Err(ParseError::Missing("offset")),
            Some(offset) => offset.parse::<i32>().map_err(ParseError::Offset)?,
        };
        let time = match components.next_back() {
            None => return Err(ParseError::Missing("time")),
            Some(time) => time.parse::<i64>().map_err(ParseError::Time)?,
        };
        let time = Time::new(time, offset);

        let email = components
            .next_back()
            .ok_or(ParseError::Missing("email"))?
            .trim_matches(|c| c == '<' || c == '>')
            .to_owned();
        let name = components.collect::<Vec<_>>().join(" ");
        Ok(Self { name, email, time })
    }
}
