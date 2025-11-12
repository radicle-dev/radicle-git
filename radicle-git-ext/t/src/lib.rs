#[cfg(test)]
#[macro_use]
extern crate assert_matches;

#[cfg(any(test, feature = "test"))]
pub mod gen;

#[cfg(test)]
mod commit;

#[cfg(any(test, feature = "test"))]
pub mod git_ref_format;

#[cfg(any(test, feature = "test"))]
pub mod repository;
