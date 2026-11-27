use core::sync::atomic::{AtomicPtr, Ordering};

use alloc_crate::{boxed::Box, sync::Arc};

use crate::decl::{DeclFile, DeclFileBox, file::DeclFileBuilder};
#[cfg(feature = "rich-unitfield")]
use crate::decl::{
	DeclNs,
	body::{Body, Field, Fields},
	id::ItemId,
	item::Item,
	items::TypeId,
};

#[cfg(feature = "rich-unitfield")]
pub fn unit_def_typeid(unit_type: TypeId) -> TypeId {
	TypeId::with_params_unchecked(DeclNs::BUILT_INS, ItemId(0xf000), [unit_type])
}

pub static PRIVATE_FILE: AtomicPtr<DeclFile> = AtomicPtr::new(core::ptr::null_mut());
pub fn init_private_file() -> &'static DeclFile {
	let mut file = DeclFileBuilder::new("@structom/private");
	#[cfg(feature = "rich-unitfield")]
	{
		let unit_type = TypeId::new_unchecked(DeclNs::GENERIC_PARAMS, ItemId(0));
		let mut fields = Fields::new(false);
		fields.add_field(Field::new("mult", unit_type.clone())).unwrap();
		let mut field = Field::new("min", unit_type.clone());
		field.set_optional(None);
		fields.add_field(field).unwrap();
		let mut field = Field::new("max", unit_type);
		field.set_optional(None);
		fields.add_field(field).unwrap();

		let mut item = Item::struct_("__UnitDef", Body::Fields(fields)).unwrap();
		item.generic_params_mut().add("UnitType").unwrap();
		file.add_item_at(item, 0xf000).unwrap();
	};

	let new = Arc::into_raw(file.finalize().unwrap().0) as *mut DeclFile;
	let ptr = match PRIVATE_FILE.compare_exchange(
		core::ptr::null_mut(),
		new,
		Ordering::AcqRel,
		Ordering::Acquire,
	) {
		Ok(_) => new,
		Err(existing) => {
			drop(unsafe { Box::from_raw(new) });
			existing
		}
	};
	unsafe { &*ptr }
}
pub fn private_file() -> &'static DeclFile {
	match unsafe { PRIVATE_FILE.load(Ordering::Acquire).as_ref() } {
		Some(f) => f,
		None => init_private_file(),
	}
}
