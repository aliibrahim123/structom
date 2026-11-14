use crate::{parse::token_cursor::*, value::Value};

pub fn parse_value(cur: &mut Cursor, is_meta_value: bool) -> Option<Value> {
	if is_meta_value {
		cur.skip();
		return Some(Value::None);
	}
	None
}
pub fn parse_map(cur: &mut Cursor, is_meta_value: bool) -> Option<Value> {
	None
}
