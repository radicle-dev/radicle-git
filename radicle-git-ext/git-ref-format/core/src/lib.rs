mod check;
pub use check::{ref_format as check_ref_format, Error, Options};

mod deriv;
pub use deriv::{Namespaced, Qualified};

pub mod lit;

pub mod name;
#[cfg(feature = "percent-encoding")]
pub use name::PercentEncode;
pub use name::{Component, RefStr, RefString};

pub mod refspec;
pub use refspec::DuplicateGlob;

#[cfg(feature = "minicbor")]
mod cbor;
#[cfg(feature = "serde")]
mod serde;
