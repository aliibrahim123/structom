#![no_std]

#[cfg(not(feature = "std"))]
extern crate alloc as alloc_crate;
#[cfg(feature = "std")]
extern crate std as alloc_crate;

pub mod decl;
mod error;
pub mod profile;
mod utils;
pub mod value;

#[cfg(feature = "parse")]
pub mod parse;
