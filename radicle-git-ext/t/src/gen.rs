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
