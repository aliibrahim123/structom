# structom javascript library
the official package for working with structom for the javascript language

## structom
structom (StructuredAtoms) is a lightweight general data exchange format designed for universal applications, from small human readable object files to large scale data serialization.

structom has 3 different forms for data representation:
- object notation: consize human readable systax for data manipulated by humans.
- binary objects: effecient direct form for shcemaless data.
- serialized structs: flattern form for performant data serialization.

structom provide additional rich data structures (tagged unions), supports both schema and schemaless data, and provide support for user defined erased metadata for richer data representation.

structom is designed to be very versatile and expressive, while remaining efficient and performant, adapting for any need from high level rich data notation to low level effecient serialization.

read more about the structom format in its [specification](../spec/index.md).

## `Value`
```typescript
export interface UUID {
	type: 'uuid',
	value: Uint8Array
};
export interface Dur {
	type: 'dur',
	value: bigint
}
export type Value = 
	boolean | number | string | bigint | Date | UUID | Dur | Array<Value> | Map<Value, Value>;
```
`Value`: a structom value, can be:
- `boolean`: represent a `bool` type.
- `number`: represent a `u8`, `u16`, `u32`, `i8`, `i16`, `i32`, `f32`, `f64` type.
- `bigint`: represent a `u64`, `i64`, `vuint`, `vint`, `bint` type.
- `string`: represent a `string` type.
- `Date`: represent a `inst`, `instN`, type.
- `UUID`: represent a `uuid` type, a 16 byte `Uint8Array` array.
- `Dur`: represent a `dur` type, a `bigint` nanosecond value.
- `Array`: represent a `arr` type.
- `Map`: represent a `map`, `struct`, `enum` type.

## `encode` / `decode`
```typescript
export function encode(value: Value): Uint8Array;
export function decode(data: ArrayBuffer): Value;
```
`encode`: encode a given `Value` into its binary representation.    
`decode`: decode a given binary data into a `Value`.

this functions expect / insert a header before the encoded data, specifing `any` type.

## codegen support
this package also export a collection of encoding and decoding utilities used by the generated serialization code, not intended to be used directly.