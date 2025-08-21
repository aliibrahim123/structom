pub fn get_char(str: &str, ind: usize) -> Option<char> {
	str.as_bytes().get(ind).map(|b| *b as char)
}

pub fn while_matching(source: &str, ind: usize, pred: fn(char) -> bool) -> usize {
	source
		.get(ind..)
		.unwrap_or("")
		.find(|c| !pred(c))
		.map(|i| ind + i)
		.unwrap_or(source.len())
}
