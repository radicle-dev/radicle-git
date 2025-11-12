#![cfg_attr(feature = "nightly", feature(try_trait_v2))]

pub mod ops;
pub mod result;

pub type Void = std::convert::Infallible;

pub mod prelude {
    use super::*;

    pub use super::Void;
    pub use ops::{FromResidual, Try};
    pub use result::ResultExt;
}
