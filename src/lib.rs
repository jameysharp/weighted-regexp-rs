#[cfg(test)]
#[macro_use]
extern crate quickcheck;

extern crate num_traits;

pub mod core;
pub mod grammars;
pub mod weights;

#[doc(inline)]
pub use core::AnyRegex;
#[doc(inline)]
pub use grammars::{empty, is, many};
#[doc(inline)]
pub use weights::recognize::{has_match, Match};
