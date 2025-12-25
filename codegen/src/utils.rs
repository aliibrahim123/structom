use std::fmt::Write;
use structom::encoding::encode_vuint;

/// add identation of certain depth
pub fn add_ident(str: &mut String, depth: usize) {
	for _ in 0..depth {
		str.push('\t');
	}
}
pub fn vuint_to_bytes(nb: u64) -> Vec<u8> {
	let mut data = Vec::new();
	encode_vuint(&mut data, nb);
	data
}

pub mod errors {
	use std::fmt::Display;

	pub fn create_dir<T>(path: &impl Display) -> impl FnOnce(T) -> String {
		move |_| format!("unable to create directory \"{path}\"")
	}
	pub fn remove_dir<T>(path: &impl Display) -> impl FnOnce(T) -> String {
		move |_| format!("unable to remove directory \"{path}\"")
	}
	pub fn read_dir<T>(path: &impl Display) -> impl FnOnce(T) -> String {
		move |_| format!("unable to read directory \"{path}\"")
	}
	pub fn read_file<T>(path: &impl Display) -> impl FnOnce(T) -> String {
		move |_| format!("unable to read file \"{path}\"")
	}
	pub fn write_file<T>(path: &impl Display) -> impl FnOnce(T) -> String {
		move |_| format!("unable to write file \"{path}\"")
	}
}

static mut SIZE_IND_COUNTER: u64 = 0;
pub fn new_size_ind() -> u64 {
	unsafe {
		SIZE_IND_COUNTER += 1;
		SIZE_IND_COUNTER
	}
}

pub fn encode_header(source: &mut String, file: &str, typeid: u64) {
	for byte in vuint_to_bytes(file.len() as u64) {
		write!(source, "0x{byte:02x}, ").unwrap();
	}
	for byte in file.as_bytes() {
		write!(source, "0x{byte:02x}, ").unwrap();
	}
	for byte in vuint_to_bytes(typeid) {
		write!(source, "0x{byte:02x}, ").unwrap();
	}
}
