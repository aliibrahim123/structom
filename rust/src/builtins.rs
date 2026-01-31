use std::{collections::HashMap, sync::LazyLock};

macro_rules! define_builtins {
    ($(($name:literal, $const_name:ident, $id:expr)),* $(,)?) => {
        $( pub const $const_name: u16 = $id; )*

        pub static BUILT_INS_IDS: LazyLock<HashMap<&'static str, u16>> =
			LazyLock::new(|| HashMap::from([
                $(($name, $const_name)),*
			]));

        pub static BUILT_INS_NAMES: LazyLock<HashMap<u16, &'static str>> =
            LazyLock::new(|| HashMap::from([
                $(($const_name, $name)),*
			]));
    };
}

define_builtins![
	("any", ANY_TYPEID, 0x01),
	("bool", BOOL_TYPEID, 0x08),
	("u8", U8_TYPEID, 0x10),
	("u16", U16_TYPEID, 0x11),
	("u32", U32_TYPEID, 0x12),
	("u64", U64_TYPEID, 0x13),
	("i8", I8_TYPEID, 0x14),
	("i16", I16_TYPEID, 0x15),
	("i32", I32_TYPEID, 0x16),
	("i64", I64_TYPEID, 0x17),
	("f32", F32_TYPEID, 0x18),
	("f64", F64_TYPEID, 0x19),
	("vuint", VUINT_TYPEID, 0x1c),
	("vint", VINT_TYPEID, 0x1d),
	("buint", BUINT_TYPEID, 0x1e),
	("bint", BINT_TYPEID, 0x1f),
	("str", STR_TYPEID, 0x20),
	("arr", ARR_TYPEID, 0x22),
	("map", MAP_TYPEID, 0x23),
	("inst", INST_TYPEID, 0x30),
	("instN", INSTN_TYPEID, 0x31),
	("dur", DUR_TYPEID, 0x32),
	("uuid", UUID_TYPEID, 0x33),
];
