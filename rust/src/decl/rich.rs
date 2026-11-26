use core::fmt::Display;
#[cfg(feature = "rich-unitfield")]
use core::ops::Index;

#[cfg(feature = "rich-bitfield")]
use crate::decl::{DeclProv, id::FieldId, resolve_item};
#[cfg(feature = "rich-unitfield")]
use crate::utils::HashMap;
use alloc_crate::vec::Vec;
use lean_string::LeanStr;
use thin_vec::ThinVec;

use crate::{
	decl::{
		AssignError, body::Body, id::ItemId, id::check_ident, items::TypeId, union::Union,
	},
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
	pub fn add(
		&mut self, name: &str, params: impl IntoIterator<Item = Value>,
	) -> Result<(), AssignError> {
		self.add_lean(LeanStr::from(name), params.into_iter().collect())
	}
	pub(crate) fn add_lean(
		&mut self, name: LeanStr, params: ThinVec<Value>,
	) -> Result<(), AssignError> {
		check_ident(&name)?;
		self.0.push(MetadataItem { name, params });
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
pub enum NotationError {
	Duplicate,
	Disallowed,
	Invalid,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum BitFieldAllow {
	#[default]
	No,
	Yes,
	ForUnion,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct RichCapabilities {
	#[cfg(feature = "rich-bitfield")]
	allow_bitfield: BitFieldAllow,
	#[cfg(feature = "rich-bitfield")]
	pub(crate) bitfield_size: u16,
	#[cfg(feature = "rich-unitfield")]
	allow_unitfield: bool,
	#[cfg(feature = "rich-unitfield")]
	unitfield_target: UnitFieldTarget,
}

#[derive(Debug, Clone)]
pub struct RichNotation {
	pub(crate) caps: RichCapabilities,
	forms: ThinVec<Form>,
}
impl RichNotation {
	pub(crate) fn new(caps: RichCapabilities) -> Self {
		Self { caps, forms: ThinVec::new() }
	}

	pub fn mark_true(&mut self) {
		if self.forms.len() == 0 {
			self.forms.push(Form::Marker);
		}
	}
	pub fn is_rich(&self) -> bool {
		!self.forms.is_empty()
	}

	pub fn add_reader(&mut self, name: &str) -> Result<(), NotationError> {
		if self.forms.iter().any(|f| matches!(f, Form::Reader(ot) if name == ot.0)) {
			return Err(NotationError::Duplicate);
		}
		self.forms.push(Form::Reader(Reader::from(name)));
		Ok(())
	}
	#[cfg(feature = "rich-bitfield")]
	pub fn add_bitfield(
		&mut self, mut bitfield: BitField, fit_container: bool,
	) -> Result<(), (BitField, NotationError)> {
		if self.caps.allow_bitfield == BitFieldAllow::No {
			return Err((bitfield, NotationError::Disallowed));
		}
		if bitfield.size > self.caps.bitfield_size
			|| !bitfield.fields().is_empty()
				&& self.caps.allow_bitfield == BitFieldAllow::ForUnion
		{
			return Err((bitfield, NotationError::Invalid));
		}
		if self.forms.iter().any(|form| matches!(form, Form::BitField(_))) {
			return Err((bitfield, NotationError::Duplicate));
		}
		if fit_container {
			bitfield.size = self.caps.bitfield_size;
		}
		self.forms.push(Form::BitField(bitfield));
		Ok(())
	}
	#[cfg(feature = "rich-unitfield")]
	pub fn add_unitfield(&mut self) -> Result<&mut UnitField, NotationError> {
		if !self.caps.allow_unitfield {
			return Err(NotationError::Disallowed);
		}
		if self.forms.iter().any(|form| matches!(form, Form::UnitField(_))) {
			return Err(NotationError::Duplicate);
		}
		self.forms.push(Form::UnitField(UnitField::new(self.caps.unitfield_target)));
		let Some(Form::UnitField(unitfield)) = self.forms.last_mut() else {
			unreachable!()
		};
		Ok(unitfield)
	}

	pub fn has_reader(&self, name: &str) -> bool {
		self.forms.iter().any(|form| matches!(form, Form::Reader(r) if r.0 == name))
	}
	#[cfg(feature = "rich-bitfield")]
	pub fn get_bitfield(&self) -> Option<&BitField> {
		self.forms.iter().find_map(|form| match form {
			Form::BitField(bf) => Some(bf),
			_ => None,
		})
	}
	#[cfg(feature = "rich-unitfield")]
	pub fn get_unitfield(&self) -> Option<&UnitField> {
		self.forms.iter().find_map(|form| match form {
			Form::UnitField(uf) => Some(uf),
			_ => None,
		})
	}

	pub fn form_count(&self) -> usize {
		self.forms.len()
	}
	pub fn forms(&self) -> &[Form] {
		&self.forms
	}
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum RichTarget<'a> {
	Body(&'a Body),
	Union(&'a Union),
	Other,
}

#[cfg(feature = "rich-bitfield")]
fn set_bitfield_caps(target: RichTarget, caps: &mut RichCapabilities) {
	match target {
		RichTarget::Body(Body::Tagged(typeid)) if typeid.is_builtin() => {
			(caps.allow_bitfield, caps.bitfield_size) = match typeid.item() {
				ItemId::U8 => (BitFieldAllow::Yes, 8),
				ItemId::U16 => (BitFieldAllow::Yes, 16),
				ItemId::U32 => (BitFieldAllow::Yes, 32),
				ItemId::U64 => (BitFieldAllow::Yes, 64),
				ItemId::ARR if typeid.params()[0] == TypeId::U8 => {
					(BitFieldAllow::Yes, typeid.params()[1].item().value())
				}
				_ => (BitFieldAllow::No, 0),
			}
		}
		RichTarget::Union(union)
			if union.variants().iter().all(|var| var.body().is_none()) =>
		{
			caps.allow_bitfield = BitFieldAllow::ForUnion;
			caps.bitfield_size = union.tag_type().bits() as u16;
		}
		_ => {}
	}
}
#[cfg(feature = "rich-unitfield")]
fn set_unitfield_caps(target: RichTarget, caps: &mut RichCapabilities) {
	if let RichTarget::Body(Body::Tagged(typeid)) = target
		&& typeid.is_builtin()
	{
		(caps.allow_unitfield, caps.unitfield_target) = match typeid.item() {
			ItemId::U8 | ItemId::U16 | ItemId::U32 | ItemId::U64 => {
				(true, UnitFieldTarget::Uint)
			}
			ItemId::I8 | ItemId::I16 | ItemId::I32 | ItemId::I64 => {
				(true, UnitFieldTarget::Int)
			}
			ItemId::F16 | ItemId::F32 | ItemId::F64 => (true, UnitFieldTarget::Float),
			ItemId::BINT => (true, UnitFieldTarget::Bint),
			ItemId::DEC => (true, UnitFieldTarget::Dec),
			_ => (false, UnitFieldTarget::Uint),
		}
	}
}
pub(crate) fn caps_of(target: RichTarget) -> RichCapabilities {
	let mut caps = RichCapabilities::default();
	#[cfg(feature = "rich-bitfield")]
	set_bitfield_caps(target, &mut caps);
	#[cfg(feature = "rich-unitfield")]
	set_unitfield_caps(target, &mut caps);
	caps
}

#[derive(Debug, Clone)]
pub enum Form {
	Marker,
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
	Atomic(u16),
	Typed(TypeId),
}

#[cfg(feature = "rich-bitfield")]
#[derive(Debug, Clone)]
pub struct BitFieldField {
	name: LeanStr,
	id: FieldId,
	type_: BitFieldType,
}
#[cfg(feature = "rich-bitfield")]
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
#[derive(Debug, Clone)]
pub struct BitField {
	size: u16,
	top: u16,
	fields: Vec<BitFieldField>,
}

#[cfg(feature = "rich-bitfield")]
impl BitField {
	pub fn new(size: u16) -> Self {
		Self { size, top: 0, fields: Vec::new() }
	}
	pub fn add(
		&mut self, name: &str, type_: BitFieldType, prov: &dyn DeclProv,
	) -> Result<FieldId, AssignError> {
		self.add_lean(LeanStr::from(name), type_, prov)
	}
	pub(crate) fn add_lean(
		&mut self, name: LeanStr, type_: BitFieldType, prov: &dyn DeclProv,
	) -> Result<FieldId, AssignError> {
		check_ident(&name).map_err(|_| AssignError::InvalidName)?;
		if self.fields.iter().any(|field| field.name == name) {
			return Err(AssignError::DuplicateName);
		}

		let size = match &type_ {
			BitFieldType::Atomic(size) => *size,
			BitFieldType::Typed(typeid) if *typeid == TypeId::BOOL => 1,
			BitFieldType::Typed(typeid) => {
				let item = resolve_item(typeid, prov)
					.map_err(|_| AssignError::UnresolvedType)?;
				if let Some(bitfield) = item.rich_notation().get_bitfield() {
					bitfield.size()
				} else {
					return Err(AssignError::InvalidType);
				}
			}
		};

		if self.top as u32 + size as u32 > self.size as u32 {
			return Err(AssignError::FullCapacity);
		}

		self.top += size;
		let id = FieldId(self.fields.len() as u16);
		self.fields.push(BitFieldField::new(name, id, type_));
		Ok(id)
	}
	pub fn size(&self) -> u16 {
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

#[derive(Debug, Default, Clone, Copy)]
enum UnitFieldTarget {
	#[default]
	Uint,
	Int,
	Float,
	Bint,
	Dec,
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
pub struct UnitNames<'a>(&'a [LeanStr]);
#[cfg(feature = "rich-unitfield")]
impl UnitNames<'_> {
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
impl Index<usize> for UnitNames<'_> {
	type Output = str;
	fn index(&self, index: usize) -> &Self::Output {
		&self.0[index].as_str()
	}
}

#[cfg(feature = "rich-unitfield")]
impl<'a> IntoIterator for UnitNames<'a> {
	type Item = &'a str;
	type IntoIter =
		core::iter::Map<core::slice::Iter<'a, LeanStr>, fn(&'a LeanStr) -> &'a str>;
	fn into_iter(self) -> Self::IntoIter {
		self.0.iter().map(|s| s.as_str())
	}
}

#[cfg(feature = "rich-unitfield")]
fn check_unit_value(value: &Value, target: UnitFieldTarget) -> Result<(), AssignError> {
	Ok(())
}

#[cfg(feature = "rich-unitfield")]
impl Unit {
	pub fn new<'a>(names: impl IntoIterator<Item = &'a str>, mult: Value) -> Self {
		let names = names.into_iter().map(LeanStr::from).collect();
		Self { names, mult, min: None, max: None }
	}
	pub fn with_bounds<'a>(
		names: impl IntoIterator<Item = &'a str>, mult: Value, min: Option<Value>,
		max: Option<Value>,
	) -> Self {
		Self { names: names.into_iter().map(LeanStr::from).collect(), mult, min, max }
	}
	pub fn names(&self) -> UnitNames<'_> {
		UnitNames(&self.names)
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
#[derive(Debug, Clone)]
pub struct UnitField {
	target: UnitFieldTarget,
	units: Vec<Unit>,
	name_index: HashMap<LeanStr, u32>,
}
#[cfg(feature = "rich-unitfield")]
impl UnitField {
	fn new(target: UnitFieldTarget) -> Self {
		Self { target, units: Vec::new(), name_index: HashMap::default() }
	}
	pub fn add(&mut self, unit: Unit) -> Result<(), AssignError> {
		check_unit_value(&unit.mult, self.target)?;
		if let Some(min) = &unit.min {
			check_unit_value(min, self.target)?;
		}
		if let Some(max) = &unit.max {
			check_unit_value(max, self.target)?;
		}

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
