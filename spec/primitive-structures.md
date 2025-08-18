# primative structures
```rust
struct MyStruct {
	a: i32,
	b: str,
	c: bool
}

enum MyEnum {
	a,
	b { v: int }
}
```
primitive structures are used to define custom data types, they allow rich data representation schemaless data can not achieve.

each structure has a name associated with it, and a typeid used in binary format.

typeids are incremented according to the order of definition, and continue after hardcoded ones.

```rust
struct Struct1 {/* ... */}     // typeid = 0
struct Struct2 {/* ... */}     // typeid = 1
struct Struct3 [5] {/* ... */} // typeid = 5
struct Struct4 {/* ... */}     // typeid = 6
```

## structs
```rust
struct MyStruct {
	a: i32,
	b: str,
	c: bool
}
```
structs are the simplist form of structures, they are collections of named values.

they are called structs, classes, objects in other languages.

### declaration
```
"struct" identifier ["[" nb "]"] struct_def
```
structs declaration consists of the struct name followed by its fields definition.

optionally, it can have a hardcoded typeid defined in brackets after the name `[nb]`
```rust
struct WithHardcodedId [3] {/* ... */}
```

structs can be inlined, defined in fields type identifiers, this is only allowed in declarations.
```
"struct" struct_def
```
```rust
struct Parent {
	child: struct {/* ... */}
}
```

### fields definition
```
struct_def = "{" field_def ("," field_def)* [","] "}"
field_name = identifier | str
field_def = ["[" nb "]"] field_name ["?"] ":" typeid
```

structs can have one or more fields separated by commas, each field is a named value of specified type.

field names can be a normal identifier, or a string.

fields can explicitly define their tag used in binary format through syntax `[nb]`.

tags are incremented according to the order of definition, continuing after hardcoded ones.

fields can be optional, by adding a `?` after the name.

```rust
struct MyStruct {
	a: i32,
	"emoji_😀": str,
	// tag = 4
	[4] c: bool
	//optional
	d?: u64
}
// tag: a = 0, b = 1, c = 4, d = 5
```

### value notation
```
struct_value = [typeid] fields_value
fields_value = "{" (field_name ":" value [","])+ "}"
```

struct values are written in a simmilar way to structs declarations.

they are written with thier typeid followed by the fields and their respective values.

the typeid can be ommited if the struct type is inferable.

fields can be written in any order.

```rust
MyStruct {
	a: 1,
	c: true,
	"emoji_😀": "hello",
}
```

### value encoding
```
+-------------+--------+
| field_count | fields |
+-------------+--------+
|   varuint   |   N    |
+-------------+--------+

field
+---------+-------------+--------+
| header  |     len     | value  |
+---------+-------------+--------+
| varuint | 0 / varuint |   N    |
+---------+-------------+--------+

header
+--------+------+
|   tag  | mlen |
+--------+------+
|   un   |  u3  |
+--------+------+
```
structs values are encoded by a varuint specifing the number of encoded fields, followed by the fields values.

each field consists of a header, an optional varuint specifing the length of the value, followed by the value.

the header is a varuint, it consists of a field (`mlen`) specifing the length of value in the 3 least significant bits, and a variant length tag taking the rest of the bits.

the values of `mlen`:
- `000`: value is 1 byte.
- `001`: value is 2 bytes.
- `010`: value is 4 bytes.
- `011`: value is 8 bytes.
- `100`: value is sized by bytes according to `len` field after the header.
- `101` - `111`: reserved.

fields can be encoded in any order, and undefined tags are skipped.

## enums
```rust
enum MyEnum { A, B, C }
```
enums are tagged unions, they represent data of variad structures determined by a discriminant.

### declaration
```
enum = "enum" identifier ["[" nb "]"] enum_def
enum_def = "{" variant_def ("," variant_def)* [","] "}"
```
enums declaration consists of the enum name followed by its variants definition.

optionally, it can have a hardcoded typeid defined in brackets after the name `[nb]`.
```rust
enum WithHardcodedId [3] {/* ... */}
``` 

enums can be inlined, defined in fields type identifiers, this is only allowed in declarations.
```
"enum" enum_def
```
```rust
struct Struct {
	value: enum {/* ... */}
}
```

### variants definition
```
variant_def = ["[" nb "]"] identifier struct_def
```
an enum can hold one or more variants separated by commas, each variant has a name and an associated tag that can be explicitly defined with `[nb]`.

variants tags are incremented according to the order of definition, continuing after hardcoded ones.

```rust
enum MyEnum {
	A,     // tag = 0
	B,     // tag = 1
	[5] C, // tag = 5
	D      // tag = 6
}
```

variants can optionally have additional data in struct format.
```rust
enum MyEnum {
	A, 
	B { v: i32 }
}
```

### value notation
```
[typeid "."] identifier [fields_value | tuple_value]
```
enums values are written by their variants name followed by their fields if defined in struct format.

if the enum type can not be infered, the used variant must be prefixed by the enum typeid followed by a dot.

```rust
B { v: 1 }
Namespace.MyEnum.A
```

### value encoding
```
+--------+--------+
|  tag   | fields |
+--------+--------+
| varuint| 0 / N  |
+--------+--------+
```
enum values are encoded by a varuint specifing the tag of the variant, optionally followed by the variant fields if defined in the struct value encoding.