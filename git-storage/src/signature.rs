// Copyright © 2022 The Radicle Link Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UserInfo {
    /// Provided `name` of the user.
    pub name: String,
    /// Proivded `email` of the user. Note that this does not
    /// necessarily have to be an email, but will be used as the email
    /// field in the [`git2::Signature`].
    pub email: String,
}

impl UserInfo {
    /// Obtain the [`git2::Signature`] for this `UserInfo`.
    pub fn signature(&self) -> Result<git2::Signature, git2::Error> {
        git2::Signature::now(&self.name, &self.email)
    }
}
