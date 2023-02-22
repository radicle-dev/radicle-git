// Copyright © 2021 The Radicle Link Contributors
//
// This file is part of radicle-link, distributed under the GPLv3 with Radicle
// Linking Exception. For full terms see the included LICENSE file.

use std::convert::TryFrom;

use ::serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    refspec::{PatternStr, PatternString},
    Namespaced,
    Qualified,
    RefStr,
    RefString,
};

impl<'de: 'a, 'a> Deserialize<'de> for &'a RefStr {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer)
            .and_then(|s: &str| Self::try_from(s).map_err(de::Error::custom))
    }
}

impl<'a> Serialize for &'a RefStr {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RefString {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer)
            .and_then(|x: String| Self::try_from(x).map_err(de::Error::custom))
    }
}

impl Serialize for RefString {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for &'a PatternStr {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer)
            .and_then(|s: &str| Self::try_from(s).map_err(de::Error::custom))
    }
}

impl<'a> Serialize for &'a PatternStr {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for PatternString {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer)
            .and_then(|x: String| Self::try_from(x).map_err(de::Error::custom))
    }
}

impl Serialize for PatternString {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Qualified<'static> {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).and_then(|s: String| {
            let s = RefString::try_from(s).map_err(de::Error::custom)?;
            s.qualified()
                .ok_or_else(|| de::Error::custom("not a qualified ref"))
                .map(|q| q.into_owned())
        })
    }
}

impl Serialize for Qualified<'_> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Namespaced<'static> {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).and_then(|s: String| {
            let s = RefString::try_from(s).map_err(de::Error::custom)?;
            s.to_namespaced()
                .ok_or_else(|| de::Error::custom("not a namespaced ref"))
                .map(|ns| ns.into_owned())
        })
    }
}

impl Serialize for Namespaced<'_> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}
