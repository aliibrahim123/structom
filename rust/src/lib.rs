#![no_std]

#[cfg(not(feature = "std"))]
extern crate alloc as alloc_crate;
#[cfg(feature = "std")]
extern crate std as alloc_crate;

mod error;
mod utils;

#[cfg(feature = "parse")]
pub mod parse;
