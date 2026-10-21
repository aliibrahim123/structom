#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Value {
	#[default]
	None,
	Fillter([u8; 31]),
}
