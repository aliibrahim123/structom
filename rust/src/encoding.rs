//! functions and types related to encoding and decoding.
//!
//! in addition to the visible items, this module exports undocumented helper functions used by the generated serialization code that are not intended to be used directly.

mod any;
mod general;
mod item;
mod nb;
mod rich;

#[doc(hidden)]
pub use any::*;
#[doc(hidden)]
pub use general::*;
#[doc(hidden)]
pub use item::skip_field;
#[doc(hidden)]
pub use nb::*;
#[doc(hidden)]
pub use rich::*;

use crate::{DeclProvider, Value, encoding::item::decode_item};

/// encode a given [`Value`] into its binary representation.
///
/// this function insert a header of empty `decl_path` to implicitly specify `any` type.
pub fn encode(value: &Value) -> Vec<u8> {
	// make decl_path empty string, so type is any implicitly
	let mut data = vec![0];
	encode_any(&mut data, value);
	data
}

/// decode a given binary data into a [`Value`].
///
/// the data must start with a header specifing the path to the declaration file and the root value typeid.
///
/// if the `decl_path` field is an empty string, the type is `any` implicitly.
///
/// it returns `None` if the data is invalid, or if there is unused space at the end of the input.
pub fn decode(data: &[u8], provider: &dyn DeclProvider) -> Option<Value> {
	let mut ind = 0;

	let decl_path = decode_str(data, &mut ind)?;

	// implicit any type if not decleration file specified
	let value = if decl_path.is_empty() {
		decode_any(data, &mut ind)?

	// else explicit type is required
	} else {
		let rootid = decode_vuint(data, &mut ind)? as u16;
		let item = provider.load(&decl_path).ok()?.get_by_id(rootid)?;
		decode_item(data, &mut ind, item, provider)?
	};

	// ensure all data is decoded
	if ind != data.len() {
		return None;
	}
	Some(value)
}

/// trait for types that can be serialized and deserialized.
///
/// this trait is automatically implemented for every generated type in generated serialization code.
pub trait Serialized
where
	Self: Sized,
{
	/// encode the type value into its binary representation.
	///
	/// this function insert the coresponding header at the beginning.
	fn encode(&self) -> Vec<u8>;

	/// encode the type value into its binary representation into the end of a given buffer.
	///
	/// this function does not insert any header.
	fn encode_inline(&self, data: &mut Vec<u8>);

	/// decode a type value from its binary representation.
	///
	/// this function expect the data to only contain the encoded value with its corresponding header.
	///
	/// it returns `None` on errors.
	fn decode(data: &[u8]) -> Option<Self>;

	/// decode a type value from its binary representation.
	///
	/// this function is same as `decode` except that it expect only the encoded data not its header.
	fn decode_headless(data: &[u8]) -> Option<Self>;

	/// decode a type value from its binary representation at the specified index in the given buffer.
	///
	/// this function expect only the encoded data, and allows additional data after the value.
	///
	/// it advances the index, and returns `None` on errors.
	fn decode_inline(data: &[u8], ind: &mut usize) -> Option<Self>;
}
