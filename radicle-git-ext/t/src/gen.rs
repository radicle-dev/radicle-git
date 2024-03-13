// Copyright © 2019-2022 The Radicle Foundation <hello@radicle.foundation>
//
// This file is part of radicle-git, distributed under the GPLv3 with Radicle
// Linking Exception. For full terms see the included LICENSE file.

//! Provides proptest generators

use proptest::strategy::Strategy;

pub mod commit;
pub mod urn;

pub fn alphanumeric() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_]+"
}

pub fn alpha() -> impl Strategy<Value = String> {
    "[a-zA-Z]+"
}
