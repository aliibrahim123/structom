mod any;
mod general;
mod item;
mod nb;
mod rich;

pub use any::*;
pub use general::*;
pub use nb::*;
pub use rich::*;

use crate::{DeclProvider, Value, encoding::item::decode_item};

pub fn encode(value: &Value) -> Vec<u8> {
	// make decl_path empty string, so type is any implicitly
	let mut data = vec![0];
	encode_any(&mut data, value);
	data
}
pub fn decode(data: &[u8], provider: &dyn DeclProvider) -> Option<Value> {
	let mut ind = 0;

	let decl_path = decode_str(data, &mut ind)?;

	// implicit any type if not decleration file specified
	let value = if decl_path.is_empty() {
		decode_any(data, &mut ind)?

	// else explicit type is required
	} else {
		let rootid = decode_vuint(data, &mut ind)? as u16;
		let item = provider.get_by_name(&decl_path)?.get_by_id(rootid)?;
		decode_item(data, &mut ind, item, provider)?
	};

	// ensure all data is decoded
	if ind != data.len() {
		return None;
	}
	Some(value)
}
