#[cfg(test)]
#[macro_use]
extern crate quickcheck;

extern crate num_traits;

pub mod weights;
pub mod core;

#[doc(inline)]
pub use core::{empty, is, many, AnyRegex};
#[doc(inline)]
pub use weights::recognize::{has_match, Match};
