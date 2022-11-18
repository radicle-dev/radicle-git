use std::{convert::TryFrom, str};

use git_ext::Oid;
use git_ref_format::{component, lit, Qualified, RefStr, RefString};

use crate::git::{refstr_join, Author};

/// The static information of a [`git2::Tag`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tag {
    /// A light-weight git tag.
    Light {
        /// The Object ID for the `Tag`, i.e the SHA1 digest.
        id: Oid,
        /// The reference name for this `Tag`.
        name: RefString,
    },
    /// An annotated git tag.
    Annotated {
        /// The Object ID for the `Tag`, i.e the SHA1 digest.
        id: Oid,
        /// The Object ID for the object that is tagged.
        target: Oid,
        /// The reference name for this `Tag`.
        name: RefString,
        /// The named author of this `Tag`, if the `Tag` was annotated.
        tagger: Option<Author>,
        /// The message with this `Tag`, if the `Tag` was annotated.
        message: Option<String>,
    },
}

impl Tag {
    /// Get the `Oid` of the tag, regardless of its type.
    pub fn id(&self) -> Oid {
        match self {
            Self::Light { id, .. } => *id,
            Self::Annotated { id, .. } => *id,
        }
    }

    /// Return the fully qualified `Tag` refname,
    /// e.g. `refs/tags/release/v1`.
    pub fn refname(&self) -> Qualified {
        lit::refs_tags(self.name()).into()
    }

    fn name(&self) -> &RefString {
        match &self {
            Tag::Light { name, .. } => name,
            Tag::Annotated { name, .. } => name,
        }
    }
}

pub mod error {
    use std::str;

    use git_ref_format::RefString;
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum FromTag {
        #[error(transparent)]
        RefFormat(#[from] git_ref_format::Error),
        #[error(transparent)]
        Utf8(#[from] str::Utf8Error),
    }

    #[derive(Debug, Error)]
    pub enum FromReference {
        #[error(transparent)]
        FromTag(#[from] FromTag),
        #[error(transparent)]
        Git(#[from] git2::Error),
        #[error("the refname '{0}' did not begin with 'refs/tags'")]
        NotQualified(String),
        #[error("the refname '{0}' did not begin with 'refs/tags'")]
        NotTag(RefString),
        #[error(transparent)]
        RefFormat(#[from] git_ref_format::Error),
        #[error(transparent)]
        Utf8(#[from] str::Utf8Error),
    }
}

impl TryFrom<&git2::Tag<'_>> for Tag {
    type Error = error::FromTag;

    fn try_from(tag: &git2::Tag) -> Result<Self, Self::Error> {
        let id = tag.id().into();
        let target = tag.target_id().into();
        let name = {
            let name = str::from_utf8(tag.name_bytes())?;
            RefStr::try_from_str(name)?.to_ref_string()
        };
        let tagger = tag.tagger().map(Author::try_from).transpose()?;
        let message = tag
            .message_bytes()
            .map(str::from_utf8)
            .transpose()?
            .map(|message| message.into());

        Ok(Tag::Annotated {
            id,
            target,
            name,
            tagger,
            message,
        })
    }
}

impl TryFrom<&git2::Reference<'_>> for Tag {
    type Error = error::FromReference;

    fn try_from(reference: &git2::Reference) -> Result<Self, Self::Error> {
        let name = {
            let name = str::from_utf8(reference.name_bytes())?;
            RefStr::try_from_str(name)?
                .qualified()
                .ok_or_else(|| error::FromReference::NotQualified(name.to_string()))?
        };

        let (_refs, tags, c, cs) = name.non_empty_components();

        if tags == component::TAGS {
            match reference.peel_to_tag() {
                Ok(tag) => Tag::try_from(&tag).map_err(error::FromReference::from),
                // If we get an error peeling to a tag _BUT_ we also have confirmed the
                // reference is a tag, that means we have a lightweight tag,
                // i.e. a commit SHA and name.
                Err(err)
                    if err.class() == git2::ErrorClass::Object
                        && err.code() == git2::ErrorCode::InvalidSpec =>
                {
                    let commit = reference.peel_to_commit()?;
                    Ok(Tag::Light {
                        id: commit.id().into(),
                        name: refstr_join(c, cs),
                    })
                },
                Err(err) => Err(err.into()),
            }
        } else {
            Err(error::FromReference::NotTag(name.into()))
        }
    }
}
