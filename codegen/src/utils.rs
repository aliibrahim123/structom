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
