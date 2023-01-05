// Copyright © 2022 The Radicle Git Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#[cfg(test)]
const GIT_PLATINUM: &str = "../data/git-platinum";

#[cfg(test)]
mod file_system;

#[cfg(test)]
mod source;

#[cfg(test)]
mod branch;

#[cfg(test)]
mod code_browsing;

#[cfg(test)]
mod commit;

#[cfg(test)]
mod diff;

#[cfg(test)]
mod last_commit;

#[cfg(test)]
mod namespace;

#[cfg(test)]
mod reference;

#[cfg(test)]
mod rev;

#[cfg(test)]
mod submodule;

#[cfg(test)]
mod threading;
