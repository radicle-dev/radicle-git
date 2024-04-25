use std::{
    fmt,
    num::ParseIntError,
    str::{self, FromStr},
};

use thiserror::Error;

/// The data for indicating authorship of an action within a
/// [`crate::commit::Commit`].
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

    fn from_components<'a>(cs: &mut impl Iterator<Item = &'a str>) -> Result<Self, ParseError> {
        let offset = match cs.next() {
            None => Err(ParseError::Missing("offset")),
            Some(offset) => Self::parse_offset(offset).map_err(ParseError::Offset),
        }?;
        let time = match cs.next() {
            None => return Err(ParseError::Missing("time")),
            Some(time) => time.parse::<i64>().map_err(ParseError::Time)?,
        };
        Ok(Self::new(time, offset))
    }

    fn parse_offset(offset: &str) -> Result<i32, ParseIntError> {
        // The offset is in the form of timezone offset,
        // e.g. +0200, -0100.  This needs to be converted into
        // minutes. The first two digits in the offset are the
        // number of hours in the offset, while the latter two
        // digits are the number of minutes in the offset.
        let tz_offset = offset.parse::<i32>()?;
        let hours = tz_offset / 100;
        let minutes = tz_offset % 100;
        Ok(hours * 60 + minutes)
    }
}

impl FromStr for Time {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_components(&mut s.split(' ').rev())
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
        let hours = self.offset.abs() / 60;
        let minutes = self.offset.abs() % 60;
        write!(f, "{} {}{:0>2}{:0>2}", self.seconds, sign, hours, minutes)
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
}

impl FromStr for Author {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Splitting the string in 4 subcomponents is expected to give back the
        // following iterator entries: timezone offset, time, email, and name
        let mut components = s.rsplitn(4, ' ');
        let time = Time::from_components(&mut components)?;
        let email = components
            .next()
            .ok_or(ParseError::Missing("email"))?
            .trim_matches(|c| c == '<' || c == '>')
            .to_owned();
        let name = components.next().ok_or(ParseError::Missing("name"))?;
        Ok(Self {
            name: name.to_owned(),
            email: email.to_owned(),
            time,
        })
    }
}
