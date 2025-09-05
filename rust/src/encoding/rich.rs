use chrono::{DateTime, TimeDelta, Timelike, Utc};

use crate::encoding::{decode_i64, decode_u32, encode_i64, encode_u32};

#[inline]
pub fn encode_uuid(data: &mut Vec<u8>, value: &[u8; 16]) {
	data.extend_from_slice(value);
}
#[inline]
pub fn decode_uuid(data: &[u8], ind: &mut usize) -> Option<[u8; 16]> {
	let value = data.get(*ind..*ind + 16)?.try_into().ok()?;
	*ind += 16;
	Some(value)
}

#[inline]
pub fn encode_inst(data: &mut Vec<u8>, value: &DateTime<Utc>) {
	encode_i64(data, value.timestamp_millis());
}
#[inline]
pub fn encode_instN(data: &mut Vec<u8>, value: &DateTime<Utc>) {
	encode_i64(data, value.timestamp_millis());
	// chrono saves nanoseconds in the current second, in structom, it must be in the current millisecond
	encode_u32(data, value.nanosecond() % 1_000_000);
}

#[inline]
pub fn decode_inst(data: &[u8], ind: &mut usize) -> Option<DateTime<Utc>> {
	DateTime::from_timestamp_millis(decode_i64(data, ind)?)
}
#[inline]
pub fn decode_instN(data: &[u8], ind: &mut usize) -> Option<DateTime<Utc>> {
	DateTime::from_timestamp_millis(decode_i64(data, ind)?)?.with_nanosecond(decode_u32(data, ind)?)
}

#[inline]
pub fn encode_dur(data: &mut Vec<u8>, value: &TimeDelta) {
	encode_i64(data, value.num_nanoseconds().unwrap());
}
#[inline]
pub fn decode_dur(data: &[u8], ind: &mut usize) -> Option<TimeDelta> {
	Some(TimeDelta::nanoseconds(decode_i64(data, ind)?))
}
