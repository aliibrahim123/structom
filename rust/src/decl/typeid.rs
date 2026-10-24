use core::hash::{Hash, Hasher};

use lean_string::LeanStr;
use thin_vec::ThinVec;

use crate::decl::{ConstructError, DeclNs, items::ItemId, rich::Metadata};

#[derive(Debug, Clone)]
pub struct TypeId {
	ns: DeclNs,
	item: ItemId,
	params: ThinVec<TypeId>,
	metadata: Metadata,
}
macro_rules! declare_builtin_typeids {
	[$($name:ident),*] => {
		$(pub const $name: TypeId = TypeId::new(DeclNs::BUILT_INS, ItemId::$name);)*
	};
}
impl TypeId {
	pub const fn new(ns: DeclNs, item: ItemId) -> Self {
		Self { ns, item, params: ThinVec::new(), metadata: Metadata::new_const() }
	}
	pub fn with_params(
		ns: DeclNs, item: ItemId, params: impl IntoIterator<Item = TypeId>,
	) -> Self {
		Self { ns, item, params: params.into_iter().collect(), metadata: Metadata::new() }
	}
	declare_builtin_typeids![
		NONE, ANY, BOOL, FAR, U8, U16, U32, U64, I8, I16, I32, I64, F32, F64, F16, STR,
		MAP, LIST, ARR, INST, DUR, BINT, DEC, UUID, COLOR
	];
	pub fn arr(item: TypeId, len: u16) -> Self {
		let len = TypeId::new(DeclNs(0), ItemId(len));
		Self::with_params(DeclNs::BUILT_INS, ItemId::ARR, [item, len])
	}
	pub fn list(item: TypeId) -> Self {
		Self::with_params(DeclNs::BUILT_INS, ItemId::LIST, [item])
	}
	pub fn map(key: TypeId, value: TypeId) -> Self {
		Self::with_params(DeclNs::BUILT_INS, ItemId::MAP, [key, value])
	}
	pub fn ns(&self) -> DeclNs {
		self.ns
	}
	pub fn item(&self) -> ItemId {
		self.item
	}
	pub fn params(&self) -> &[TypeId] {
		&self.params
	}
	pub fn is_builtin(&self) -> bool {
		self.ns == DeclNs::BUILT_INS
	}
	pub fn strip_metadata(&self) -> Self {
		Self {
			ns: self.ns,
			item: self.item,
			params: self.params.clone(),
			metadata: Metadata::new_const(),
		}
	}
	pub fn metadata(&self) -> &Metadata {
		&self.metadata
	}
	pub fn metadata_mut(&mut self) -> &mut Metadata {
		&mut self.metadata
	}
}
impl Default for TypeId {
	fn default() -> Self {
		Self::NONE
	}
}
impl PartialEq for TypeId {
	fn eq(&self, other: &Self) -> bool {
		self.ns == other.ns && self.item == other.item && self.params == other.params
	}
}
impl Eq for TypeId {}
impl PartialOrd for TypeId {
	fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
		Some(self.cmp(other))
	}
}
impl Ord for TypeId {
	fn cmp(&self, other: &Self) -> core::cmp::Ordering {
		self.ns
			.cmp(&other.ns)
			.then(self.item.cmp(&other.item))
			.then_with(|| self.params.cmp(&other.params))
	}
}
impl Hash for TypeId {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.ns.hash(state);
		self.item.hash(state);
		self.params.hash(state);
	}
}

pub(crate) mod raw {
	pub const NONE: u16 = 0x00;
	pub const ANY: u16 = 0x01;
	pub const BOOL: u16 = 0x02;
	pub const FAR: u16 = 0x03;

	pub const U8: u16 = 0x10;
	pub const U16: u16 = 0x11;
	pub const U32: u16 = 0x12;
	pub const U64: u16 = 0x13;
	pub const I8: u16 = 0x14;
	pub const I16: u16 = 0x15;
	pub const I32: u16 = 0x16;
	pub const I64: u16 = 0x17;

	pub const F32: u16 = 0x18;
	pub const F64: u16 = 0x19;
	pub const F16: u16 = 0x1a;

	pub const ARR: u16 = 0x20;
	pub const LIST: u16 = 0x21;
	pub const STR: u16 = 0x22;
	pub const MAP: u16 = 0x23;

	pub const INST: u16 = 0x80;
	pub const DUR: u16 = 0x81;
	pub const BINT: u16 = 0x82;
	pub const DEC: u16 = 0x83;
	pub const UUID: u16 = 0x84;
	pub const COLOR: u16 = 0x85;
}

#[derive(Debug, Clone)]
pub struct GenericParam {
	name: LeanStr,
	id: ItemId,
}
impl GenericParam {
	pub fn name(&self) -> &str {
		self.name.as_str()
	}
	pub fn id(&self) -> ItemId {
		self.id
	}
	pub fn as_typeid(&self) -> TypeId {
		TypeId::new(DeclNs::GENERIC_PARAMS, self.id)
	}
}

#[derive(Debug, Clone)]
pub struct GenericParams(ThinVec<GenericParam>);
impl GenericParams {
	pub(crate) fn new() -> Self {
		Self(ThinVec::new())
	}
	pub fn len(&self) -> usize {
		self.0.len()
	}
	pub fn add(&mut self, name: &str) -> Result<ItemId, ConstructError> {
		if self.0.iter().any(|param| param.name == name) {
			return Err(ConstructError::DuplicateName);
		}
		let id = ItemId(self.0.len() as u16);
		self.0.push(GenericParam { name: LeanStr::from(name), id });
		Ok(id)
	}
	pub fn get_by_id(&self, id: ItemId) -> Option<&GenericParam> {
		self.0.get(id.0 as usize)
	}
	pub fn get_by_name(&self, name: &str) -> Option<&GenericParam> {
		self.0.iter().find(|param| param.name == name)
	}
	pub fn as_slice(&self) -> &[GenericParam] {
		&self.0
	}
}
