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

use std::fmt;

use serde::{Deserialize, Serialize};

use radicle_surf::vcs::git::{self, Browser, RefScope};

use crate::error::Error;

/// Branch name representation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Branch(pub(crate) String);

impl From<String> for Branch {
    fn from(name: String) -> Self {
        Self(name)
    }
}

impl From<git::Branch> for Branch {
    fn from(branch: git::Branch) -> Self {
        Self(branch.name.to_string())
    }
}

impl fmt::Display for Branch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Given a project id to a repo returns the list of branches.
///
/// # Errors
///
/// Will return [`Error`] if the project doesn't exist or the surf interaction
/// fails.
pub fn branches(browser: &Browser<'_>, filter: RefScope) -> Result<Vec<Branch>, Error> {
    let mut branches = browser
        .list_branches(filter)?
        .into_iter()
        .map(|b| Branch(b.name.name().to_string()))
        .collect::<Vec<Branch>>();

    branches.sort();

    Ok(branches)
}

/// Information about a locally checked out repository.
#[derive(Deserialize, Serialize)]
pub struct LocalState {
    /// List of branches.
    branches: Vec<Branch>,
}

/// Given a path to a repo returns the list of branches and if it is managed by
/// coco.
///
/// # Errors
///
/// Will return [`Error`] if the repository doesn't exist.
pub fn local_state(repo_path: &str, default_branch: &str) -> Result<LocalState, Error> {
    let repo = git2::Repository::open(repo_path).map_err(git::error::Error::from)?;
    let first_branch = repo
        .branches(Some(git2::BranchType::Local))
        .map_err(git::error::Error::from)?
        .filter_map(|branch_result| {
            let (branch, _) = branch_result.ok()?;
            let name = branch.name().ok()?;
            name.map(String::from)
        })
        .min()
        .ok_or(Error::NoBranches)?;

    let repo = git::Repository::new(repo_path)?;

    let browser = match Browser::new(&repo, git::Branch::local(default_branch)) {
        Ok(browser) => browser,
        Err(_) => Browser::new(&repo, git::Branch::local(&first_branch))?,
    };

    let mut branches = browser
        .list_branches(RefScope::Local)?
        .into_iter()
        .map(|b| Branch(b.name.name().to_string()))
        .collect::<Vec<Branch>>();

    branches.sort();

    Ok(LocalState { branches })
}
