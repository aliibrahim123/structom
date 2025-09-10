use std::{collections::HashMap, hash::Hash};

use crate::encoding::nb::{decode_vuint, encode_vuint};

// in_field = true, omit length field
#[inline]
pub fn encode_bool(data: &mut Vec<u8>, value: bool) {
	data.push(value as u8);
}
#[inline]
pub fn decode_bool(data: &[u8], ind: &mut usize) -> Option<bool> {
	let value = *data.get(*ind)? != 0;
	*ind += 1;
	Some(value)
}

#[inline]
pub fn encode_u8_arr(data: &mut Vec<u8>, value: &[u8]) {
	encode_vuint(data, value.len() as u64);
	data.extend_from_slice(value);
}
#[inline]
pub fn decode_u8_arr(data: &[u8], ind: &mut usize) -> Option<Vec<u8>> {
	let len = decode_vuint(data, ind)? as usize;
	let value = data.get(*ind..*ind + len)?.to_vec();
	*ind += len;
	Some(value)
}

#[inline]
pub fn encode_str(data: &mut Vec<u8>, value: &str) {
	encode_u8_arr(data, value.as_bytes());
}
#[inline]
pub fn decode_str(data: &[u8], ind: &mut usize) -> Option<String> {
	Some(String::from_utf8(decode_u8_arr(data, ind)?).ok()?)
}

#[inline]
pub fn encode_arr<T>(
	data: &mut Vec<u8>, value: &[T], in_field: bool, item_fn: impl Fn(&mut Vec<u8>, &T) -> (),
) {
	if !in_field {
		encode_vuint(data, value.len() as u64)
	}
	for v in value {
		item_fn(data, v);
	}
}
#[inline]
pub fn decode_arr<T>(
	data: &[u8], ind: &mut usize, in_field: bool, item_fn: impl Fn(&[u8], &mut usize) -> Option<T>,
) -> Option<Vec<T>> {
	let len = decode_vuint(data, ind)? as usize;
	if in_field {
		let start_ind = *ind;
		let mut vec = Vec::new();
		while *ind < start_ind + len {
			vec.push(item_fn(data, ind)?);
		}
		Some(vec)
	} else {
		let mut vec = Vec::with_capacity(len);
		for _ in 0..len {
			vec.push(item_fn(data, ind)?);
		}
		Some(vec)
	}
}

#[inline]
pub fn encode_map<K, V>(
	data: &mut Vec<u8>, value: &HashMap<K, V>, in_field: bool,
	key_fn: impl Fn(&mut Vec<u8>, &K) -> (), val_fn: impl Fn(&mut Vec<u8>, &V) -> (),
) {
	if !in_field {
		encode_vuint(data, value.len() as u64)
	}
	for (k, v) in value {
		key_fn(data, k);
		val_fn(data, v);
	}
}
#[inline]
pub fn decode_map<K: Eq + Hash, V>(
	data: &[u8], ind: &mut usize, in_field: bool, key_fn: impl Fn(&[u8], &mut usize) -> Option<K>,
	val_fn: impl Fn(&[u8], &mut usize) -> Option<V>,
) -> Option<HashMap<K, V>> {
	let len = decode_vuint(data, ind)? as usize;
	if in_field {
		let start_ind = *ind;
		let mut map = HashMap::new();
		while *ind < start_ind + len {
			let k = key_fn(data, ind)?;
			let v = val_fn(data, ind)?;
			map.insert(k, v);
		}
		Some(map)
	} else {
		let mut map = HashMap::with_capacity(len);
		for _ in 0..len {
			let k = key_fn(data, ind)?;
			let v = val_fn(data, ind)?;
			map.insert(k, v);
		}
		Some(map)
	}
}
