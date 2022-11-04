// Copyright © 2019-2020 The Radicle Foundation <hello@radicle.foundation>
//
// This file is part of radicle-link, distributed under the GPLv3 with Radicle
// Linking Exception. For full terms see the included LICENSE file.

use std::{
    convert::TryFrom,
    fmt::{self, Display},
    ops::Deref,
    str::FromStr,
};

/// Serializable [`git2::Oid`]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Oid(git2::Oid);

#[cfg(feature = "serde")]
mod serde_impls {
    use super::*;
    use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};

    impl Serialize for Oid {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            self.0.to_string().serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for Oid {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct OidVisitor;

            impl<'de> Visitor<'de> for OidVisitor {
                type Value = Oid;

                fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    write!(f, "a hexidecimal git2::Oid")
                }

                fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    s.parse().map_err(serde::de::Error::custom)
                }
            }

            deserializer.deserialize_str(OidVisitor)
        }
    }
}

impl Deref for Oid {
    type Target = git2::Oid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<git2::Oid> for Oid {
    fn as_ref(&self) -> &git2::Oid {
        self
    }
}

impl AsRef<[u8]> for Oid {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl From<git2::Oid> for Oid {
    fn from(oid: git2::Oid) -> Self {
        Self(oid)
    }
}

impl From<Oid> for git2::Oid {
    fn from(oid: Oid) -> Self {
        oid.0
    }
}

impl Display for Oid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl TryFrom<&str> for Oid {
    type Error = git2::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse().map(Self)
    }
}

impl FromStr for Oid {
    type Err = git2::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl TryFrom<&[u8]> for Oid {
    type Error = git2::Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        git2::Oid::from_bytes(bytes).map(Self)
    }
}
