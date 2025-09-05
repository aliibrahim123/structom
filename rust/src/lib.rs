pub(crate) mod builtins;
mod declaration;
pub mod encoding;
mod errors;
mod fs_decl_provider;
mod parser;
mod stringify;
mod value;

pub use declaration::{
	DeclFile, DeclProvider, FixedSetProvider, FixedSetProviderRef, VoidProvider,
};
pub use encoding::{decode, encode};
pub use errors::Error;
pub use fs_decl_provider::{FSProvider, LoadFileError};
pub use parser::{ParseOptions, parse, parse_declaration_file};
pub use stringify::{StringifyOptions, stringify};
pub use value::{Key, Value};

#[doc(hidden)]
pub mod internal {
	pub use crate::builtins::*;
	pub use crate::declaration::{DeclItem, EnumVariant, Field, StructDef, TypeId};
}
