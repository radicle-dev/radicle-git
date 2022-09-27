// Copyright © 2022 The Radicle Link Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The [`Edit`] and [`Remove`] enums provide a mechanism for protecting updates
//! during concurrent writes to the refdb. The value is constructed with the
//! policy, sometimes providing the expected previous value of the [`Oid`]
//! target. If this policy is not satisfied then the update should be rejected.

use git_ext::Oid;

use thiserror::Error;

/// The expectation of the reference's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Edit {
    /// No requirements are made towards the current value, and the new value is
    /// set unconditionally.
    Any,
    /// The reference must exist and may have any value.
    MustExist,
    /// Create the ref only, hence the reference must not exist.
    MustNotExist,
    /// The ref _must_ exist and have the given value.
    MustExistAndMatch(Oid),
    /// The ref _may_ exist and have the given value, or may not exist at all.
    MayExistAndMatch(Oid),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Error)]
pub enum EditError {
    #[error("the reference does not exist when it was expected to exist")]
    DoesNotExist,
    #[error("the reference does exist when it was expected not to exist")]
    DoesExist,
    #[error("the reference does not match - given: '{given}' expected: '{expected}'")]
    DoesNotMatch { given: Oid, expected: Oid },
}

impl Edit {
    /// Guard against the `given` value using this [`Edit`].
    ///
    /// The function will return [`Err`] if the `given` value does not pass the
    /// policy the policy of the [`Edit`].
    ///
    /// The `given` value is presumed non-existing if set to `None`, otherwise
    /// it is presumed to be existing.
    pub fn guard(&self, given: Option<Oid>) -> Result<(), EditError> {
        use EditError::*;

        match self {
            Self::Any => Ok(()),
            Self::MustExist => {
                if given.is_none() {
                    Err(DoesNotExist)
                } else {
                    Ok(())
                }
            },
            Self::MustNotExist => {
                if given.is_some() {
                    Err(DoesExist)
                } else {
                    Ok(())
                }
            },
            Self::MustExistAndMatch(expected) => match given {
                Some(given) if &given == expected => Ok(()),
                Some(given) => Err(DoesNotMatch {
                    given,
                    expected: *expected,
                }),
                None => Err(DoesNotExist),
            },
            Self::MayExistAndMatch(expected) => match given {
                Some(given) if &given == expected => Ok(()),
                Some(given) => Err(DoesNotMatch {
                    given,
                    expected: *expected,
                }),
                None => Ok(()),
            },
        }
    }
}

/// The expectation of the existing reference's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Remove {
    /// The reference must exist and may have any value.
    MustExist,
    /// The ref _must_ exist and have the given value.
    MustExistAndMatch(Oid),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Error)]
pub enum RemoveError {
    #[error("the reference does not exist when it was expected to exist")]
    DoesNotExist,
    #[error("the reference does not match - given: '{given}' expected: '{expected}'")]
    DoesNotMatch { given: Oid, expected: Oid },
}

impl Remove {
    /// Guard against the `given` value using this [`Remove`].
    ///
    /// The function will return [`Err`] if the `given` value does not pass the
    /// policy the policy of the [`Remove`].
    ///
    /// The `given` value is presumed non-existing if set to `None`, otherwise
    /// it is presumed to be existing.
    pub fn guard(&self, given: Option<Oid>) -> Result<(), RemoveError> {
        use RemoveError::*;

        match self {
            Self::MustExist => {
                if given.is_none() {
                    Err(DoesNotExist)
                } else {
                    Ok(())
                }
            },
            Self::MustExistAndMatch(expected) => match given {
                Some(given) if &given == expected => Ok(()),
                Some(given) => Err(DoesNotMatch {
                    given,
                    expected: *expected,
                }),
                None => Err(DoesNotExist),
            },
        }
    }
}
