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

#[derive(Debug, Clone)]
pub struct MetadataItem {
	name: LeanStr,
	value: Value,
}
impl MetadataItem {
	pub fn name(&self) -> &str {
		self.name.as_str()
	}
	pub fn value(&self) -> &Value {
		&self.value
	}
}

#[derive(Debug, Clone, Default)]
pub struct Metadata(ThinVec<MetadataItem>);
impl Metadata {
	pub fn new() -> Self {
		Self::new_const()
	}
	pub(crate) const fn new_const() -> Self {
		Self(ThinVec::new())
	}
}
impl Metadata {
	pub fn len(&self) -> usize {
		self.0.len()
	}
	pub fn add(&mut self, name: &str, value: Value) -> Result<(), ConstructError> {
		self.add_lean(LeanStr::from(name), value)
	}
	pub(crate) fn add_lean(
		&mut self, name: LeanStr, value: Value,
	) -> Result<(), ConstructError> {
		check_ident(&name)?;
		self.0.push(MetadataItem { name, value });
		Ok(())
	}
	pub fn get(&self, name: &str) -> Option<&Value> {
		self.0.iter().find_map(|item| (item.name == name).then_some(&item.value))
	}
	pub fn get_multiple(&self, name: &str) -> impl Iterator<Item = &Value> {
		self.0.iter().filter_map(move |item| (item.name == name).then_some(&item.value))
	}
	pub fn remove(&mut self, name: &str) {
		self.0.retain(|item| item.name != name);
	}
	pub fn as_slice(&self) -> &[MetadataItem] {
		&self.0
	}
}
impl<'a> FromIterator<(&'a str, Value)> for Metadata {
	fn from_iter<T: IntoIterator<Item = (&'a str, Value)>>(iter: T) -> Self {
		Self(ThinVec::from_iter(
			iter.into_iter().map(|(n, value)| MetadataItem { name: n.into(), value }),
		))
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

	pub fn form_count(&self) -> usize {
		self.0.len()
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
impl Reader {
	pub fn as_str(&self) -> &str {
		self.0.as_str()
	}
}
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
	fn new(size: u8) -> Self {
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
	pub fn field_count(&self) -> usize {
		self.fields.len()
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
	fn new() -> Self {
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
	pub fn unit_count(&self) -> usize {
		self.units.len()
	}
	pub fn units(&self) -> &[Unit] {
		&self.units
	}
}
