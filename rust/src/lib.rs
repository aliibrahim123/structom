#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

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
