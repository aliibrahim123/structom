use core::hash;
use std::{
	collections::HashMap,
	hash::Hash,
	ops::{Index, IndexMut},
	slice::SliceIndex,
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
	Array(Vec<Value>),
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
	($ty:ty, $enum:ident, $var:ident) => {
		impl From<$ty> for $enum {
			fn from(v: $ty) -> Self {
				$enum::$var(v)
			}
		}
	};
	($ty:ident, $enum:ident, $var:ident, $as:ident) => {
		impl From<$ty> for $enum {
			fn from(v: $ty) -> Self {
				$enum::$var(v as $as)
			}
		}
	};
}

from_impl!(bool, Value, Bool);
from_impl!(i64, Value, Int);
from_impl!(u64, Value, Uint);
from_impl!(f64, Value, Float);
from_impl!(String, Value, Str);
from_impl!(DateTime<Utc>, Value, Inst);
from_impl!(TimeDelta, Value, Dur);

from_impl!(u8, Value, Uint, u64);
from_impl!(u16, Value, Uint, u64);
from_impl!(u32, Value, Uint, u64);
from_impl!(usize, Value, Uint, u64);
from_impl!(i8, Value, Int, i64);
from_impl!(i16, Value, Int, i64);
from_impl!(i32, Value, Int, i64);
from_impl!(isize, Value, Int, i64);
from_impl!(f32, Value, Float, f64);

from_impl!(bool, Key, Bool);
from_impl!(i64, Key, Int);
from_impl!(u64, Key, Uint);
from_impl!(String, Key, Str);
from_impl!(DateTime<Utc>, Key, Inst);
from_impl!(TimeDelta, Key, Dur);

from_impl!(u8, Key, Uint, u64);
from_impl!(u16, Key, Uint, u64);
from_impl!(u32, Key, Uint, u64);
from_impl!(usize, Key, Uint, u64);
from_impl!(i8, Key, Int, i64);
from_impl!(i16, Key, Int, i64);
from_impl!(i32, Key, Int, i64);
from_impl!(isize, Key, Int, i64);

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
		Value::Array(v.into_iter().map(|v| v.into()).collect())
	}
}
impl<K: Into<Key>, V: Into<Value>> From<HashMap<K, V>> for Value {
	fn from(m: HashMap<K, V>) -> Self {
		Value::Map(m.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
	}
}

macro_rules! try_into_impl {
	($ty:ty, $enum:ident, $var:ident) => {
		impl TryInto<$ty> for $enum {
			type Error = ();
			fn try_into(self) -> Result<$ty, Self::Error> {
				match self {
					$enum::$var(v) => Ok(v as $ty),
					_ => Err(()),
				}
			}
		}
	};
}
macro_rules! try_into_int_impl {
	($ty:ty, $enum:ident) => {
		impl TryInto<$ty> for $enum {
			type Error = ();
			fn try_into(self) -> Result<$ty, Self::Error> {
				match self {
					$enum::Int(v) => Ok(v as $ty),
					$enum::Uint(v) => Ok(v as $ty),
					_ => Err(()),
				}
			}
		}
	};
}
try_into_impl!(bool, Value, Bool);
try_into_impl!(u64, Value, Uint);
try_into_impl!(i64, Value, Int);
try_into_impl!(f64, Value, Float);
try_into_impl!(String, Value, Str);
try_into_impl!(DateTime<Utc>, Value, Inst);
try_into_impl!(TimeDelta, Value, Dur);

try_into_int_impl!(u8, Value);
try_into_int_impl!(u16, Value);
try_into_int_impl!(u32, Value);
try_into_int_impl!(usize, Value);
try_into_int_impl!(i8, Value);
try_into_int_impl!(i16, Value);
try_into_int_impl!(i32, Value);
try_into_int_impl!(isize, Value);
try_into_impl!(f32, Value, Float);

try_into_impl!(bool, Key, Bool);
try_into_impl!(u64, Key, Uint);
try_into_impl!(i64, Key, Int);
try_into_impl!(String, Key, Str);
try_into_impl!(DateTime<Utc>, Key, Inst);
try_into_impl!(TimeDelta, Key, Dur);

try_into_int_impl!(u8, Key);
try_into_int_impl!(u16, Key);
try_into_int_impl!(u32, Key);
try_into_int_impl!(usize, Key);
try_into_int_impl!(i8, Key);
try_into_int_impl!(i16, Key);
try_into_int_impl!(i32, Key);
try_into_int_impl!(isize, Key);

impl<T> TryInto<Vec<T>> for Value
where
	Value: TryInto<T>,
{
	type Error = ();
	fn try_into(self) -> Result<Vec<T>, Self::Error> {
		match self {
			Value::Array(v) => {
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
	($ty:ty, $met:ident, $enum:ident, $var:ident) => {
		impl $enum {
			pub fn $met(&self) -> Option<$ty> {
				match self {
					$enum::$var(v) => Some(*v),
					_ => None,
				}
			}
		}
	};
}
macro_rules! as_ref_impl {
	($ty:ty, $met:ident, $enum:ident, $var:ident) => {
		impl $enum {
			pub fn $met(&self) -> Option<&$ty> {
				match self {
					$enum::$var(v) => Some(v),
					_ => None,
				}
			}
		}
	};
}
macro_rules! as_mut_impl {
	($ty:ty, $met:ident, $enum:ident, $var:ident) => {
		impl $enum {
			pub fn $met(&mut self) -> Option<&mut $ty> {
				match self {
					$enum::$var(v) => Some(v),
					_ => None,
				}
			}
		}
	};
}
as_impl!(bool, as_bool, Value, Bool);
as_impl!(i64, as_int, Value, Int);
as_impl!(u64, as_uint, Value, Uint);
as_impl!(f64, as_float, Value, Float);
as_impl!(DateTime<Utc>, as_inst, Value, Inst);
as_impl!(TimeDelta, as_dur, Value, Dur);
as_ref_impl!(str, as_str, Value, Str);
as_ref_impl!([Value], as_slice, Value, Array);
as_mut_impl!(Vec<Value>, as_vec, Value, Array);
as_ref_impl!(HashMap<Key, Value>, as_map, Value, Map);
as_mut_impl!(HashMap<Key, Value>, as_map_mut, Value, Map);
as_impl!([u8; 16], as_uuid, Value, UUID);
as_ref_impl!([u8], as_bigint, Value, BigInt);

as_impl!(bool, as_bool, Key, Bool);
as_impl!(i64, as_int, Key, Int);
as_impl!(u64, as_uint, Key, Uint);
as_impl!(DateTime<Utc>, as_inst, Key, Inst);
as_impl!(TimeDelta, as_dur, Key, Dur);
as_ref_impl!(str, as_str, Key, Str);
as_impl!([u8; 16], as_uuid, Key, UUID);
as_ref_impl!([u8], as_bigint, Key, BigInt);

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
	($ty:ty, $enum:ident, $var:ident) => {
		impl PartialEq<$ty> for $enum {
			fn eq(&self, other: &$ty) -> bool {
				match self {
					$enum::$var(v) => v == other,
					_ => false,
				}
			}
		}
	};
}
macro_rules! eq_int_impl {
	($ty:ty, $enum:ident) => {
		impl PartialEq<$ty> for $enum {
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
		}
	};
}

eq_impl!(bool, Value, Bool);
eq_impl!(String, Value, Str);
eq_impl!(DateTime<Utc>, Value, Inst);
eq_impl!(TimeDelta, Value, Dur);
eq_impl!(f64, Value, Float);
eq_impl!(&str, Value, Str);

eq_int_impl!(u64, Value);
eq_int_impl!(u32, Value);
eq_int_impl!(u16, Value);
eq_int_impl!(u8, Value);
eq_int_impl!(usize, Value);
eq_int_impl!(i64, Value);
eq_int_impl!(i32, Value);
eq_int_impl!(i16, Value);
eq_int_impl!(i8, Value);
eq_int_impl!(isize, Value);

eq_impl!(bool, Key, Bool);
eq_impl!(String, Key, Str);
eq_impl!(DateTime<Utc>, Key, Inst);
eq_impl!(TimeDelta, Key, Dur);
eq_impl!(&str, Key, Str);

eq_int_impl!(u64, Key);
eq_int_impl!(u32, Key);
eq_int_impl!(u16, Key);
eq_int_impl!(u8, Key);
eq_int_impl!(usize, Key);
eq_int_impl!(i64, Key);
eq_int_impl!(i32, Key);
eq_int_impl!(i16, Key);
eq_int_impl!(i8, Key);
eq_int_impl!(isize, Key);

impl<T> PartialEq<Vec<T>> for Value
where
	Value: PartialEq<T>,
{
	fn eq(&self, other: &Vec<T>) -> bool {
		match self {
			Value::Array(a) => a == other,
			_ => false,
		}
	}
}

impl<I: SliceIndex<[Value]>> Index<I> for Value {
	type Output = <I as SliceIndex<[Value]>>::Output;
	fn index(&self, index: I) -> &Self::Output {
		match self {
			Value::Array(a) => &a[index],
			_ => panic!(),
		}
	}
}
impl<I: SliceIndex<[Value]>> IndexMut<I> for Value {
	fn index_mut(&mut self, index: I) -> &mut Self::Output {
		match self {
			Value::Array(a) => &mut a[index],
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
			Value::Array(a) => a.get(index),
			_ => None,
		}
	}
	pub fn get_by_index_mut<I: SliceIndex<[Value]>>(
		&mut self, index: I,
	) -> Option<&mut <I as SliceIndex<[Value]>>::Output> {
		match self {
			Value::Array(a) => a.get_mut(index),
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
			Value::Array(a) => Some(a.iter()),
			_ => None,
		}
	}
}
