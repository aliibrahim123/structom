use core::fmt::Display;
#[cfg(feature = "rich-unitfield")]
use core::ops::Index;

#[cfg(feature = "rich-unitfield")]
use crate::utils::HashMap;
use alloc_crate::vec::Vec;
use lean_string::LeanStr;
use thin_vec::ThinVec;

use crate::{
	decl::{ConstructError, id::check_ident, items::TypeId},
	value::Value,
};

#[derive(Debug, Clone, Default)]
pub struct Metadata(ThinVec<(LeanStr, Value)>);
impl Metadata {
	pub fn new() -> Self {
		Self::new_const()
	}
	pub(crate) const fn new_const() -> Self {
		Self(ThinVec::new())
	}
}
impl Metadata {
	pub fn add(&mut self, str: &str, value: Value) -> Result<(), ConstructError> {
		self.add_lean(LeanStr::from(str), value)
	}
	pub(crate) fn add_lean(
		&mut self, str: LeanStr, value: Value,
	) -> Result<(), ConstructError> {
		check_ident(&str)?;
		self.0.push((str, value));
		Ok(())
	}
	pub fn get(&self, str: &str) -> Option<&Value> {
		self.0.iter().find_map(|(s, v)| (s == str).then_some(v))
	}
	pub fn get_multiple(&self, str: &str) -> impl Iterator<Item = &Value> {
		self.0.iter().filter_map(move |(s, v)| (s == str).then_some(v))
	}
	pub fn remove(&mut self, str: &str) {
		self.0.retain(|(s, _)| s != str);
	}
	pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
		self.0.iter().map(|(s, v)| (s.as_str(), v))
	}
}
impl<'a> FromIterator<(&'a str, Value)> for Metadata {
	fn from_iter<T: IntoIterator<Item = (&'a str, Value)>>(iter: T) -> Self {
		Self(ThinVec::from_iter(iter.into_iter().map(|(s, v)| (LeanStr::from(s), v))))
	}
}

#[derive(Debug, Default)]
pub struct RichNotation(ThinVec<Form>);
impl RichNotation {
	pub fn new() -> Self {
		Self::default()
	}
	pub fn add_reader(&mut self, name: &str) -> Result<(), ConstructError> {
		if self.0.iter().any(|f| matches!(f, Form::Reader(other) if name == other.0)) {
			return Err(ConstructError::DuplicateNotation);
		}
		self.0.push(Form::Reader(Reader::from(name)));
		Ok(())
	}
	#[cfg(feature = "rich-bitfield")]
	pub fn add_bitfield(&mut self, size: u8) -> Result<(), ConstructError> {
		if self.0.iter().any(|form| matches!(form, Form::BitField(_))) {
			return Err(ConstructError::DuplicateNotation);
		}
		self.0.push(Form::BitField(BitField::new(size)));
		Ok(())
	}
	#[cfg(feature = "rich-unitfield")]
	pub fn add_unitfield(&mut self) -> Result<(), ConstructError> {
		if self.0.iter().any(|form| matches!(form, Form::UnitField(_))) {
			return Err(ConstructError::DuplicateNotation);
		}
		self.0.push(Form::UnitField(UnitField::new()));
		Ok(())
	}

	pub fn forms(&self) -> &[Form] {
		&self.0
	}
}

#[derive(Debug)]
pub enum Form {
	Reader(Reader),
	#[cfg(feature = "rich-bitfield")]
	BitField(BitField),
	#[cfg(feature = "rich-unitfield")]
	UnitField(UnitField),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Reader(LeanStr);
impl From<LeanStr> for Reader {
	fn from(s: LeanStr) -> Self {
		Self(s)
	}
}
impl From<&str> for Reader {
	fn from(s: &str) -> Self {
		Self(LeanStr::from(s))
	}
}
impl Display for Reader {
	fn fmt(&self, f: &mut alloc_crate::fmt::Formatter<'_>) -> alloc_crate::fmt::Result {
		write!(f, "{}", self.0)
	}
}

#[cfg(feature = "rich-bitfield")]
#[derive(Debug, Clone)]
pub enum BitFieldType {
	Atomic(u8),
	Typed(TypeId),
}

#[cfg(feature = "rich-bitfield")]
#[derive(Debug)]
pub struct BitField {
	size: u8,
	fields: Vec<(LeanStr, BitFieldType)>,
}
#[cfg(feature = "rich-bitfield")]
impl BitField {
	pub fn new(size: u8) -> Self {
		Self { size, fields: Vec::new() }
	}
	pub fn add(&mut self, name: &str, field: BitFieldType) -> Result<(), ConstructError> {
		self.add_lean(LeanStr::from(name), field)
	}
	pub(crate) fn add_lean(
		&mut self, name: LeanStr, field: BitFieldType,
	) -> Result<(), ConstructError> {
		check_ident(&name)?;
		self.fields.push((name, field));
		Ok(())
	}
	pub fn size(&self) -> u8 {
		self.size
	}
	pub fn fields(&self) -> impl Iterator<Item = (&str, &BitFieldType)> {
		self.fields.iter().map(|(name, field)| (name.as_str(), field))
	}
}

#[cfg(feature = "rich-unitfield")]
#[derive(Debug, Clone)]
pub struct Unit {
	names: Vec<LeanStr>,
	mult: Value,
	min: Option<Value>,
	max: Option<Value>,
}
#[cfg(feature = "rich-unitfield")]
#[derive(Debug, Clone)]
pub struct Names<'a>(&'a [LeanStr]);
#[cfg(feature = "rich-unitfield")]
impl Names<'_> {
	pub fn len(&self) -> usize {
		self.0.len()
	}
	pub fn get(&self, index: usize) -> Option<&str> {
		self.0.get(index).map(|s| s.as_str())
	}
	pub fn iter(&self) -> impl Iterator<Item = &str> {
		self.0.iter().map(|s| s.as_str())
	}
}
#[cfg(feature = "rich-unitfield")]
impl Index<usize> for Names<'_> {
	type Output = str;
	fn index(&self, index: usize) -> &Self::Output {
		&self.0[index].as_str()
	}
}
#[cfg(feature = "rich-unitfield")]
impl<'a> IntoIterator for Names<'a> {
	type Item = &'a str;
	type IntoIter =
		core::iter::Map<core::slice::Iter<'a, LeanStr>, fn(&'a LeanStr) -> &'a str>;
	fn into_iter(self) -> Self::IntoIter {
		self.0.iter().map(|s| s.as_str())
	}
}

#[cfg(feature = "rich-unitfield")]
impl Unit {
	pub fn new<'a>(names: impl IntoIterator<Item = &'a str>, mult: Value) -> Self {
		Self::new_lean(names.into_iter().map(LeanStr::from).collect(), mult)
	}
	pub(crate) fn new_lean(names: Vec<LeanStr>, mult: Value) -> Self {
		Self { names, mult, min: None, max: None }
	}
	pub fn names(&self) -> Names {
		Names(&self.names)
	}
	pub fn mult(&self) -> &Value {
		&self.mult
	}
	pub fn min(&self) -> Option<&Value> {
		self.min.as_ref()
	}
	pub fn max(&self) -> Option<&Value> {
		self.max.as_ref()
	}
}

#[cfg(feature = "rich-unitfield")]
#[derive(Debug)]
pub struct UnitField {
	units: Vec<Unit>,
	name_index: HashMap<LeanStr, u32>,
}
#[cfg(feature = "rich-unitfield")]
impl UnitField {
	pub fn new() -> Self {
		Self { units: Vec::new(), name_index: HashMap::default() }
	}
	pub fn add(&mut self, unit: Unit) -> Result<(), ConstructError> {
		if unit.names.iter().any(|name| self.name_index.contains_key(name)) {
			return Err(ConstructError::DuplicateName);
		}
		for name in unit.names.iter() {
			self.name_index.insert(name.clone(), self.units.len() as u32);
		}
		self.units.push(unit);
		Ok(())
	}
	pub fn units(&self) -> &[Unit] {
		&self.units
	}
}
