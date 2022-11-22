// This file is part of radicle-surf
// <https://github.com/radicle-dev/radicle-surf>
//
// Copyright (C) 2019-2020 The Radicle Team <dev@radicle.xyz>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 or
// later as published by the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::collections::{BTreeMap, BTreeSet};

use git_ref_format::{RefStr, RefString};

use crate::git::{Error, Glob, Repository};

/// A `View` represents a logical view of peer's repository in the
/// [Heartwood protocol][heartwood].
///
/// The `Id` parameter allows the specification of what the identifier
/// for a peer is in Heartwood.
///
/// The `P` parameter allows the specification of what a person looks
/// like in Heartwood, whether that be a DID document, website, etc.
///
/// [heartwood]: https://github.com/radicle-dev/heartwood
pub struct View<Id, P> {
    /// Identifier for the peer that this `View` is associated to.
    pub identifier: Id,
    /// Personal information for the peer that this `View` is associated to.
    pub person: Option<P>,
    /// All `refs/heads` and `refs/remotes` reference names.
    pub branches: BTreeSet<RefString>,
    /// All `refs/tags` reference names.
    pub tags: BTreeSet<RefString>,
    /// Any `refs/<category>` reference names, where each key in the map is a
    /// `<category>` and the set is the references under that category.
    pub references: BTreeMap<RefString, BTreeSet<RefString>>,
}

/// Construct a [`View`] for the `identifier` and `person`.
///
/// `identifier` is assumed to act as the namespace in the Heartwood storage.
///
/// `categories` is the set of git non-standard reference categories,
/// i.e. `refs/heads`, `refs/remotes`, `refs/tags`, and `refs/notes`.
pub fn view<Id, R, P>(
    repo: &Repository,
    identifier: Id,
    person: Option<P>,
    categories: impl IntoIterator<Item = RefString>,
) -> Result<View<Id, P>, Error>
where
    Id: Clone + Into<R>,
    R: AsRef<RefStr>,
{
    let namespace = identifier.clone().into();
    repo.with_namespace(&namespace.as_ref().to_ref_string(), move || {
        let branches = repo
            .branch_names(Glob::all_heads())?
            .map(|name| name.map(RefString::from))
            .collect::<Result<_, _>>()?;
        let tags = repo
            .tag_names(&Glob::all_tags())?
            .map(|name| name.map(RefString::from))
            .collect::<Result<_, _>>()?;
        let categories = categories.into_iter().fold(Glob::default(), |globs, cat| {
            globs.and(Glob::all_category(cat))
        });
        let references =
            repo.categories(&categories)?
                .try_fold(BTreeMap::new(), |mut map, r| {
                    let (cat, name) = r?;
                    map.entry(cat)
                        .and_modify(|names: &mut BTreeSet<RefString>| {
                            names.insert(name.clone());
                        })
                        .or_insert_with(|| BTreeSet::from_iter(Some(name)));
                    Ok::<_, Error>(map)
                })?;
        Ok(View {
            identifier,
            person,
            branches,
            tags,
            references,
        })
    })
}
