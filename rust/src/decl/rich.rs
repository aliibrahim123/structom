use core::fmt::Display;
#[cfg(feature = "rich-unitfield")]
use core::ops::Index;

#[cfg(feature = "rich-bitfield")]
use crate::decl::id::FieldId;
#[cfg(feature = "rich-unitfield")]
use crate::utils::HashMap;
use alloc_crate::vec::Vec;
use lean_string::LeanStr;
use thin_vec::ThinVec;

use crate::{
	decl::{AssignError, id::check_ident, items::TypeId},
	value::Value,
};

#[derive(Debug, Clone)]
pub struct MetadataItem {
	name: LeanStr,
	params: ThinVec<Value>,
}
impl MetadataItem {
	pub fn name(&self) -> &str {
		self.name.as_str()
	}
	pub fn params_count(&self) -> usize {
		self.params.len()
	}
	pub fn params(&self) -> &[Value] {
		&self.params
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
	pub fn add(&mut self, name: &str, params: ThinVec<Value>) -> Result<(), AssignError> {
		self.add_lean(LeanStr::from(name), params)
	}
	pub(crate) fn add_lean(
		&mut self, name: LeanStr, params: impl IntoIterator<Item = Value>,
	) -> Result<(), AssignError> {
		check_ident(&name)?;
		self.0.push(MetadataItem { name, params: ThinVec::from_iter(params) });
		Ok(())
	}
	pub fn get(&self, name: &str) -> Option<&MetadataItem> {
		self.0.iter().find(|item| item.name == name)
	}
	pub fn get_multiple(&self, name: &str) -> impl Iterator<Item = &MetadataItem> {
		self.0.iter().filter(move |item| item.name == name)
	}
	pub fn remove(&mut self, name: &str) {
		self.0.retain(|item| item.name != name);
	}
	pub fn as_slice(&self) -> &[MetadataItem] {
		&self.0
	}
}
impl<'a, G: IntoIterator<Item = Value>> FromIterator<(&'a str, G)> for Metadata {
	fn from_iter<T: IntoIterator<Item = (&'a str, G)>>(iter: T) -> Self {
		Self(ThinVec::from_iter(iter.into_iter().map(|(name, params)| MetadataItem {
			name: name.into(),
			params: ThinVec::from_iter(params),
		})))
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuplicateNotation;

#[derive(Debug, Default)]
pub struct RichNotation(ThinVec<Form>);
impl RichNotation {
	pub fn new() -> Self {
		Self::default()
	}
	pub fn add_reader(&mut self, name: &str) -> Result<(), DuplicateNotation> {
		if self.0.iter().any(|f| matches!(f, Form::Reader(other) if name == other.0)) {
			return Err(DuplicateNotation);
		}
		self.0.push(Form::Reader(Reader::from(name)));
		Ok(())
	}
	#[cfg(feature = "rich-bitfield")]
	pub fn add_bitfield(&mut self, size: u8) -> Result<&mut BitField, DuplicateNotation> {
		if self.0.iter().any(|form| matches!(form, Form::BitField(_))) {
			return Err(DuplicateNotation);
		}
		self.0.push(Form::BitField(BitField::new(size)));
		let Some(Form::BitField(bitfield)) = self.0.last_mut() else { unreachable!() };
		Ok(bitfield)
	}
	#[cfg(feature = "rich-unitfield")]
	pub fn add_unitfield(&mut self) -> Result<&mut UnitField, DuplicateNotation> {
		if self.0.iter().any(|form| matches!(form, Form::UnitField(_))) {
			return Err(DuplicateNotation);
		}
		self.0.push(Form::UnitField(UnitField::new()));
		let Some(Form::UnitField(unitfield)) = self.0.last_mut() else { unreachable!() };
		Ok(unitfield)
	}

	pub fn has_reader(&self, name: &str) -> bool {
		self.0.iter().any(|form| matches!(form, Form::Reader(r) if r.0 == name))
	}
	#[cfg(feature = "rich-bitfield")]
	pub fn get_bitfield(&self) -> Option<&BitField> {
		self.0.iter().find_map(|form| match form {
			Form::BitField(bf) => Some(bf),
			_ => None,
		})
	}
	#[cfg(feature = "rich-unitfield")]
	pub fn get_unitfield(&self) -> Option<&UnitField> {
		self.0.iter().find_map(|form| match form {
			Form::UnitField(uf) => Some(uf),
			_ => None,
		})
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
#[derive(Debug, Clone)]
pub struct BitFieldField {
	name: LeanStr,
	id: FieldId,
	type_: BitFieldType,
}
impl BitFieldField {
	fn new(name: LeanStr, id: FieldId, type_: BitFieldType) -> Self {
		Self { name, id, type_ }
	}
	pub fn name(&self) -> &str {
		&self.name
	}
	pub fn id(&self) -> FieldId {
		self.id
	}
	pub fn type_(&self) -> &BitFieldType {
		&self.type_
	}
}

#[cfg(feature = "rich-bitfield")]
#[derive(Debug)]
pub struct BitField {
	size: u8,
	fields: Vec<BitFieldField>,
}
#[cfg(feature = "rich-bitfield")]
impl BitField {
	fn new(size: u8) -> Self {
		Self { size, fields: Vec::new() }
	}
	pub fn add(
		&mut self, name: &str, type_: BitFieldType,
	) -> Result<FieldId, AssignError> {
		self.add_lean(LeanStr::from(name), type_)
	}
	pub(crate) fn add_lean(
		&mut self, name: LeanStr, type_: BitFieldType,
	) -> Result<FieldId, AssignError> {
		check_ident(&name)?;
		if self.fields.len() >= u16::MAX as usize {
			return Err(AssignError::FullCapacity);
		}
		let id = FieldId(self.fields.len() as u16);
		self.fields.push(BitFieldField::new(name, id, type_));
		Ok(id)
	}
	pub fn size(&self) -> u8 {
		self.size
	}
	pub fn field_by_name(&self, name: &str) -> Option<&BitFieldField> {
		self.fields.iter().find_map(|field| (field.name == name).then_some(field))
	}
	pub fn field_by_id(&self, id: FieldId) -> Option<&BitFieldField> {
		self.fields.get(id.0 as usize)
	}
	pub fn field_count(&self) -> usize {
		self.fields.len()
	}
	pub fn fields(&self) -> &[BitFieldField] {
		&self.fields
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
	pub fn names(&self) -> Names<'_> {
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
	pub fn add(&mut self, unit: Unit) -> Result<(), AssignError> {
		if unit.names.iter().any(|name| self.name_index.contains_key(name)) {
			return Err(AssignError::DuplicateName);
		}
		for name in unit.names.iter() {
			self.name_index.insert(name.clone(), self.units.len() as u32);
		}
		self.units.push(unit);
		Ok(())
	}
	pub fn unit_by_name(&self, name: &str) -> Option<&Unit> {
		self.name_index.get(name).map(|&i| &self.units[i as usize])
	}
	pub fn unit_count(&self) -> usize {
		self.units.len()
	}
	pub fn units(&self) -> &[Unit] {
		&self.units
	}
}
