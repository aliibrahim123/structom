use core::hash;
use std::{
	collections::HashMap,
	hash::Hash,
	ops::{Index, IndexMut},
	slice::SliceIndex,
	sync::LazyLock,
};

use chrono::{DateTime, TimeDelta, Utc};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
	Bool(bool),
	Int(i64),
	Uint(u64),
	BigInt(Vec<u8>),
	Float(f64),
	Str(String),
	Inst(DateTime<Utc>),
	Dur(TimeDelta),
	UUID([u8; 16]),
	Arr(Vec<Value>),
	Map(HashMap<Key, Value>),
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum Key {
	Bool(bool),
	Int(i64),
	Uint(u64),
	BigInt(Vec<u8>),
	Inst(DateTime<Utc>),
	Dur(TimeDelta),
	UUID([u8; 16]),
	Str(String),
}

impl Default for Value {
	fn default() -> Self {
		Value::Uint(0)
	}
}
impl Default for Key {
	fn default() -> Self {
		Key::Uint(0)
	}
}

macro_rules! is_impl {
	($enum:ident, $(($ty:ident, $met:ident)),+) => {
		$(pub fn $met(&self) -> bool {
			match self {
				$enum::$ty(_) => true,
				_ => false,
			}
		})+
	};
}

static ENUM_VARIANT_KEY: LazyLock<Key> = LazyLock::new(|| Key::Str("$enum_variant".to_string()));
impl Value {
	is_impl!(Value, (Bool, is_bool), (Uint, is_uint), (Int, is_int), (Str, is_str));
	is_impl!(Value, (BigInt, is_bigint), (Float, is_float), (Inst, is_inst), (Dur, is_dur));
	is_impl!(Value, (UUID, is_uuid), (Arr, is_array), (Map, is_map));

	pub fn into_key(self) -> Key {
		self.try_into().unwrap()
	}
	pub fn enum_variant(&self) -> Option<&str> {
		match self {
			Value::Map(map) => map.get(&ENUM_VARIANT_KEY).and_then(|v| v.as_str()),
			_ => None,
		}
	}
}
impl Key {
	is_impl!(Key, (Bool, is_bool), (Uint, is_uint), (Int, is_int), (Str, is_str));
	is_impl!(Key, (BigInt, is_bigint), (Inst, is_inst), (Dur, is_dur), (UUID, is_uuid));
}

impl TryFrom<Value> for Key {
	type Error = ();
	fn try_from(value: Value) -> Result<Self, Self::Error> {
		match value {
			Value::Bool(b) => Ok(Key::Bool(b)),
			Value::Int(i) => Ok(Key::Int(i)),
			Value::Uint(i) => Ok(Key::Uint(i)),
			Value::BigInt(i) => Ok(Key::BigInt(i)),
			Value::Str(s) => Ok(Key::Str(s)),
			Value::Inst(i) => Ok(Key::Inst(i)),
			Value::Dur(d) => Ok(Key::Dur(d)),
			Value::UUID(u) => Ok(Key::UUID(u)),
			_ => Err(()),
		}
	}
}
impl From<Key> for Value {
	fn from(key: Key) -> Self {
		match key {
			Key::Bool(b) => Value::Bool(b),
			Key::Int(i) => Value::Int(i),
			Key::Uint(i) => Value::Uint(i),
			Key::BigInt(i) => Value::BigInt(i),
			Key::Str(s) => Value::Str(s),
			Key::Inst(i) => Value::Inst(i),
			Key::Dur(d) => Value::Dur(d),
			Key::UUID(u) => Value::UUID(u),
		}
	}
}

macro_rules! from_impl {
	($enum:ident, $(($ty:ty, $var:ident)),+) => {
		$(impl From<$ty> for $enum {
			fn from(v: $ty) -> Self {
				$enum::$var(v)
			}
		})+
	};
	($enum:ident, $var:ident, $as:ident, [$($ty:ident),+]) => {
		$(impl From<$ty> for $enum {
			fn from(v: $ty) -> Self {
				$enum::$var(v as $as)
			}
		})+
	};
}

from_impl!(Value, (bool, Bool), (i64, Int), (u64, Uint), (f64, Float));
from_impl!(Value, (String, Str), (DateTime<Utc>, Inst), (TimeDelta, Dur));
from_impl!(Value, ([u8; 16], UUID));

from_impl!(Value, Uint, u64, [u8, u16, u32, usize]);
from_impl!(Value, Int, i64, [i8, i16, i32, isize]);
from_impl!(Value, Float, f64, [f32]);

from_impl!(Key, (bool, Bool), (i64, Int), (u64, Uint), (String, Str));
from_impl!(Key, (DateTime<Utc>, Inst), (TimeDelta, Dur), ([u8; 16], UUID));

from_impl!(Key, Uint, u64, [u8, u16, u32, usize]);
from_impl!(Key, Int, i64, [i8, i16, i32, isize]);

impl From<&str> for Value {
	fn from(s: &str) -> Self {
		Value::Str(s.to_string())
	}
}
impl From<&str> for Key {
	fn from(s: &str) -> Self {
		Key::Str(s.to_string())
	}
}

impl<T: Into<Value>> From<Vec<T>> for Value {
	fn from(v: Vec<T>) -> Self {
		Value::Arr(v.into_iter().map(|v| v.into()).collect())
	}
}
impl<K: Into<Key>, V: Into<Value>> From<HashMap<K, V>> for Value {
	fn from(m: HashMap<K, V>) -> Self {
		Value::Map(m.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
	}
}

macro_rules! try_into_impl {
	($enum:ident, $(($ty:ty, $var:ident)),+) => {
		$(impl TryInto<$ty> for $enum {
			type Error = ();
			fn try_into(self) -> Result<$ty, Self::Error> {
				match self {
					$enum::$var(v) => Ok(v as $ty),
					_ => Err(()),
				}
			}
		})+
	};
}
macro_rules! try_into_int_impl {
	($enum:ident, [$($ty:ty),+]) => {
		$(impl TryInto<$ty> for $enum {
			type Error = ();
			fn try_into(self) -> Result<$ty, Self::Error> {
				match self {
					$enum::Int(v) => Ok(v as $ty),
					$enum::Uint(v) => Ok(v as $ty),
					_ => Err(()),
				}
			}
		})+
	};
}
try_into_impl!(Value, (bool, Bool), (u64, Uint), (i64, Int), (f64, Float), (f32, Float));
try_into_impl!(Value, (String, Str), (DateTime<Utc>, Inst), (TimeDelta, Dur));
try_into_impl!(Value, ([u8; 16], UUID));

try_into_int_impl!(Value, [u8, u16, u32, usize, i8, i16, i32, isize]);

try_into_impl!(Key, (bool, Bool), (u64, Uint), (i64, Int), (String, Str));
try_into_impl!(Key, (DateTime<Utc>, Inst), (TimeDelta, Dur), ([u8; 16], UUID));
try_into_int_impl!(Key, [u8, u16, u32, usize, i8, i16, i32, isize]);

impl<T> TryInto<Vec<T>> for Value
where
	Value: TryInto<T>,
{
	type Error = ();
	fn try_into(self) -> Result<Vec<T>, Self::Error> {
		match self {
			Value::Arr(v) => {
				let mut vec = Vec::<T>::with_capacity(v.len());
				for item in v {
					match item.try_into() {
						Ok(v) => vec.push(v),
						Err(_) => return Err(()),
					}
				}
				Ok(vec)
			}
			_ => Err(()),
		}
	}
}
impl<K, V> TryInto<HashMap<K, V>> for Value
where
	Key: TryInto<K>,
	Value: TryInto<V>,
	K: Eq + hash::Hash,
{
	type Error = ();
	fn try_into(self) -> Result<HashMap<K, V>, Self::Error> {
		match self {
			Value::Map(m) => {
				let mut map = HashMap::<K, V>::with_capacity(m.len());
				for (k, v) in m {
					match (k.try_into(), v.try_into()) {
						(Ok(k), Ok(v)) => _ = map.insert(k, v),
						_ => return Err(()),
					}
				}
				Ok(map)
			}
			_ => Err(()),
		}
	}
}

macro_rules! as_impl {
	($enum:ident, $(($ty:ty, $met:ident, $var:ident)),+) => {
		$(pub fn $met(&self) -> Option<$ty> {
			match self {
				$enum::$var(v) => Some(*v),
				_ => None,
			}
		})+
	};
}
macro_rules! as_ref_impl {
	($enum:ident, $(($ty:ty, $met:ident, $var:ident)),+) => {
		$(pub fn $met(&self) -> Option<&$ty> {
			match self {
				$enum::$var(v) => Some(v),
				_ => None,
			}
		})+
	};
}
macro_rules! as_mut_impl {
	($enum:ident, $(($ty:ty, $met:ident, $var:ident)),+) => {
		$(pub fn $met(&mut self) -> Option<&mut $ty> {
			match self {
				$enum::$var(v) => Some(v),
				_ => None,
			}
		})+
	};
}

impl Value {
	as_impl!(Value, (bool, as_bool, Bool), (i64, as_int, Int), (u64, as_uint, Uint));
	as_impl!(Value, (f64, as_float, Float), (DateTime<Utc>, as_inst, Inst));
	as_impl!(Value, (TimeDelta, as_dur, Dur), ([u8; 16], as_uuid, UUID));
	as_ref_impl!(Value, (str, as_str, Str), ([Value], as_slice, Arr));
	as_ref_impl!(Value, ([u8], as_bigint, BigInt), (HashMap<Key, Value>, as_map, Map));
	as_mut_impl!(Value, (Vec<Value>, as_vec_mut, Arr), (HashMap<Key, Value>, as_map_mut, Map));
}

impl Key {
	as_impl!(Key, (bool, as_bool, Bool), (i64, as_int, Int), (DateTime<Utc>, as_inst, Inst));
	as_impl!(Key, (TimeDelta, as_dur, Dur), (u64, as_uint, Uint), ([u8; 16], as_uuid, UUID));
	as_ref_impl!(Key, (str, as_str, Str), ([u8], as_bigint, BigInt));
}

impl PartialEq<Key> for Value {
	fn eq(&self, other: &Key) -> bool {
		match (self, other) {
			(Value::Bool(a), Key::Bool(b)) => a == b,
			(Value::Int(a), Key::Int(b)) => a == b,
			(Value::Uint(a), Key::Uint(b)) => a == b,
			(Value::BigInt(a), Key::BigInt(b)) => a == b,
			(Value::Str(a), Key::Str(b)) => a == b,
			(Value::Inst(a), Key::Inst(b)) => a == b,
			(Value::Dur(a), Key::Dur(b)) => a == b,
			(Value::UUID(a), Key::UUID(b)) => a == b,
			_ => false,
		}
	}
}

macro_rules! eq_impl {
	($enum:ident, $(($ty:ty, $var:ident)),+) => {
		$(impl PartialEq<$ty> for $enum {
			fn eq(&self, other: &$ty) -> bool {
				match self {
					$enum::$var(v) => v == other,
					_ => false,
				}
			}
		})+
	};
}
macro_rules! eq_int_impl {
	($enum:ident, [$($ty:ty),+]) => {
		$(impl PartialEq<$ty> for $enum {
			fn eq(&self, other: &$ty) -> bool {
				match self {
					$enum::Uint(v) => *v as $ty == *other,
					$enum::Int(v) => *v as $ty == *other,
					_ => false,
				}
			}
		}
		impl PartialEq<$ty> for &$enum {
			fn eq(&self, other: &$ty) -> bool {
				match self {
					$enum::Uint(v) => *v as $ty == *other,
					$enum::Int(v) => *v as $ty == *other,
					_ => false,
				}
			}
		})+
	};
}

eq_impl!(Value, (bool, Bool), (DateTime<Utc>, Inst), (TimeDelta, Dur), (f64, Float));
eq_impl!(Value, (&str, Str), (String, Str));

eq_int_impl!(Value, [u8, u16, u32, u64, usize]);
eq_int_impl!(Value, [i8, i16, i32, i64, isize]);

eq_impl!(Key, (bool, Bool), (DateTime<Utc>, Inst), (TimeDelta, Dur), (String, Str), (&str, Str));

eq_int_impl!(Key, [u8, u16, u32, u64, usize]);
eq_int_impl!(Key, [i8, i16, i32, i64, isize]);

impl<T> PartialEq<Vec<T>> for Value
where
	Value: PartialEq<T>,
{
	fn eq(&self, other: &Vec<T>) -> bool {
		match self {
			Value::Arr(a) => a == other,
			_ => false,
		}
	}
}

impl<I: SliceIndex<[Value]>> Index<I> for Value {
	type Output = <I as SliceIndex<[Value]>>::Output;
	fn index(&self, index: I) -> &Self::Output {
		match self {
			Value::Arr(a) => &a[index],
			_ => panic!(),
		}
	}
}
impl<I: SliceIndex<[Value]>> IndexMut<I> for Value {
	fn index_mut(&mut self, index: I) -> &mut Self::Output {
		match self {
			Value::Arr(a) => &mut a[index],
			_ => panic!(),
		}
	}
}
impl Index<&Key> for Value {
	type Output = Value;
	fn index(&self, index: &Key) -> &Self::Output {
		match self {
			Value::Map(m) => m.get(index).unwrap(),
			_ => panic!(),
		}
	}
}
impl IndexMut<&Key> for Value {
	fn index_mut(&mut self, index: &Key) -> &mut Self::Output {
		match self {
			Value::Map(m) => match m.contains_key(index) {
				true => m.get_mut(index).unwrap(),
				false => m.entry(index.clone()).or_insert(Value::default()),
			},
			_ => panic!(),
		}
	}
}
impl Value {
	pub fn get_by_index<I: SliceIndex<[Value]>>(
		&self, index: I,
	) -> Option<&<I as SliceIndex<[Value]>>::Output> {
		match self {
			Value::Arr(a) => a.get(index),
			_ => None,
		}
	}
	pub fn get_by_index_mut<I: SliceIndex<[Value]>>(
		&mut self, index: I,
	) -> Option<&mut <I as SliceIndex<[Value]>>::Output> {
		match self {
			Value::Arr(a) => a.get_mut(index),
			_ => None,
		}
	}
	pub fn get_by_key(&self, key: &Key) -> Option<&Value> {
		match self {
			Value::Map(m) => m.get(key),
			_ => None,
		}
	}
	pub fn get_by_key_mut(&mut self, key: &Key) -> Option<&mut Value> {
		match self {
			Value::Map(m) => m.get_mut(key),
			_ => None,
		}
	}

	pub fn into_iter(&self) -> Option<impl Iterator<Item = &Value>> {
		match self {
			Value::Arr(a) => Some(a.iter()),
			_ => None,
		}
	}
}
