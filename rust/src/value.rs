use core::hash;
use std::{
	collections::HashMap,
	hash::Hash,
	time::{Duration, Instant},
};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
	Bool(bool),
	Int(i64),
	Uint(u64),
	BigInt(Vec<u8>),
	Float(f64),
	Str(String),
	Inst(Instant),
	Dur(Duration),
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
	Inst(Instant),
	Dur(Duration),
	UUID([u8; 16]),
	Str(String),
}

impl Value {
	pub fn as_str(&self) -> Option<&str> {
		match self {
			Value::Str(s) => Some(&s),
			_ => None,
		}
	}
	pub fn as_slice(&self) -> Option<&[Value]> {
		match self {
			Value::Array(v) => Some(&v[..]),
			_ => None,
		}
	}
}

impl Key {
	pub fn as_str(&self) -> Option<&str> {
		match self {
			Key::Str(s) => Some(&s),
			_ => None,
		}
	}
}

impl Default for Value {
	fn default() -> Self {
		Value::Int(0)
	}
}
impl Default for Key {
	fn default() -> Self {
		Key::Int(0)
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
	($ty:ident, $enum:ident, $var:ident) => {
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
from_impl!(Instant, Value, Inst);
from_impl!(Duration, Value, Dur);

from_impl!(u8, Value, Uint, u64);
from_impl!(u16, Value, Uint, u64);
from_impl!(u32, Value, Uint, u64);
from_impl!(i8, Value, Int, i64);
from_impl!(i16, Value, Int, i64);
from_impl!(i32, Value, Int, i64);
from_impl!(f32, Value, Float, f64);

from_impl!(bool, Key, Bool);
from_impl!(i64, Key, Int);
from_impl!(u64, Key, Uint);
from_impl!(String, Key, Str);
from_impl!(Instant, Key, Inst);
from_impl!(Duration, Key, Dur);

from_impl!(u8, Key, Uint, u64);
from_impl!(u16, Key, Uint, u64);
from_impl!(u32, Key, Uint, u64);
from_impl!(i8, Key, Int, i64);
from_impl!(i16, Key, Int, i64);
from_impl!(i32, Key, Int, i64);

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
	($ty:ident, $enum:ident, $var:ident) => {
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
	($ty:ident, $enum:ident) => {
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
try_into_impl!(Instant, Value, Inst);
try_into_impl!(Duration, Value, Dur);

try_into_int_impl!(u8, Value);
try_into_int_impl!(u16, Value);
try_into_int_impl!(u32, Value);
try_into_int_impl!(i8, Value);
try_into_int_impl!(i16, Value);
try_into_int_impl!(i32, Value);
try_into_impl!(f32, Value, Float);

try_into_impl!(bool, Key, Bool);
try_into_impl!(u64, Key, Uint);
try_into_impl!(i64, Key, Int);
try_into_impl!(String, Key, Str);
try_into_impl!(Instant, Key, Inst);
try_into_impl!(Duration, Key, Dur);

try_into_int_impl!(u8, Key);
try_into_int_impl!(u16, Key);
try_into_int_impl!(u32, Key);
try_into_int_impl!(i8, Key);
try_into_int_impl!(i16, Key);
try_into_int_impl!(i32, Key);

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
