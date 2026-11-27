use crate::{
	decl::items::TypeId,
	parse::{head::Scope, token_cursor::*},
	value::Value,
};

pub fn parse_value(
	cur: &mut Cursor, typeid: &TypeId, scope: &mut Scope,
) -> Option<Value> {
	None
}
pub fn parse_map(cur: &mut Cursor, is_meta_value: bool) -> Option<Value> {
	None
}
