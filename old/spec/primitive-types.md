# primitive types
```javascript
{
	int: 3,
	float: 3.14,
	bool: true,
	str: "a"
}
```
primitive types are built in types that represent the general simple data types found in every laguage.

## fixed size integers
```rust
1
-129
0x07Ff_07Ff
```
fixed size integers are whole numbers, signed or unsigned.

they come in deffirent sizes: 8 bit, 16 bit, 32 bit and 64 bit.

type | bits | bytes | sign     | min                  | max                  |
---- | ---- | ----- | -------- | -------------------- | -------------------- |
u8   | 8    | 1     | unsigned | 0                    | 255                  |
i8   | 8    | 1     | signed   | -128                 | 127                  |
u16  | 16   | 2     | unsigned | 0                    | 65535                |
i16  | 16   | 2     | signed   | -32768               | 32767                |
u32  | 32   | 4     | unsigned | 0                    | 4294967295           |
i32  | 32   | 4     | signed   | -2147483648          | 2147483647           |
u64  | 64   | 8     | unsigned | 0                    | 18446744073709551615 |
i64  | 64   | 8     | signed   | -9223372036854775808 | 9223372036854775807  |

### value notation
```
uint_value = nb 
int_value = ["+" | "-"] nb
```
fixed size integers are witten with number literals.

### binary encoding
```
+--------+  +---------+
|   id   |  |  value  |
+--------+  +---------+
|  0x1x  |  | ux / ix |
+--------+  +---------+
```
fixed size integers are encoded in 2 complement little endian in 1, 2, 4 or 8 bytes depending on the type.

type | id   | bytes |
---- | ---- | ----- |
 u8  | 0x10 | 1     |
 u16 | 0x11 | 2     |
 u32 | 0x12 | 4     |
 u64 | 0x13 | 8     |
 i8  | 0x14 | 1     |
 i16 | 0x15 | 2     |
 i32 | 0x16 | 4     |
 i64 | 0x17 | 8     |

## variable size integers
```
123
-123
1234567890_1234567890bint
```
variable size integers are integers, signed or unsigned, that takes a variable number of bytes.

`vint` and `vuint` are variable size integers that correspond to 64 bit signed and unsigned integers, taking 1 to 9 bytes.

`bint` is variable size integer of arbitrary sizes, taking as many bytes as needed.

### value notation
```
vint_value = signed_nb
vuint_value = nb
bint_value = signed_nb "bint"
```
variable size integers are written with number literals.

the suffix is required for bigint.

if a number literal value in not inferable and with no suffix, it is assumed to be a `vuint` then `vint` value.

### binary encoding
```
vint / vuint
+--------+  +--------+
|   id   |  | value  |
+--------+  +--------+
|  0x1x  |  |  vint  |
+--------+  +--------+

bint        value
+--------+  +--------+--------+
|   id   |  |  size  | value  |
+--------+  +--------+--------+
|  0x1e  |  |  vint  | x size |
+--------+  +--------+--------+
```
`vint` / `vuint` are encoded in 2 complement in LEB128 encoding, taking as many bytes as needed, max 10 bytes.

`bint` is encoded in 2 complement and in little endian encoded as an array of bytes.

in case a `bint` is encoded in a field value, the size section is omitted and the length is infered from the `len` section in the field encoding.

type  | id   |
----- | ---- |
vuint | 0x1c |
vint  | 0x1d |
bint  | 0x1e |

## floating point numbers
```rust
123.456
3.3e-12
```
floating point numbers are IEEE 754 compliant numbers.

they can be of half, single or double precision.

### value notation
```
float_exp = ("e" | "E") ["+" | "-"] dec_part
float_value =
	["+" | "-"] (dec_part | [dec_part] "." dec_part) [float_exp] | 
	("nan" | (["+" | "-"] "inf"))

```
floating point numbers are written with float literals.

float literals are written in decimal base, with optional exponent.

float literals can be NaN, positive or negative infinity.

if a number literal value in not inferable, it is assumed to be `f32` value.

### binary encoding
```
+--------+  +--------+
|   id   |  | value  |
+--------+  +--------+
|  0x1x  |  |   fn   |
+--------+  +--------+
```
floating point numbers are encoded in IEEE 754 encoding in little endian, in 2, 4 or 8 bytes depending on the type.

type  | id   | bytes |
----- | ---- | ----- |
f32   | 0x18 | 4     |
f64   | 0x19 | 8     |

## boolean
```rust
true
false
```
boolean is a type that can take only two values: `true` or `false`.

### value notation
```
"true" | "false"
```

### binary encoding
```
+--------+  +--------+
|   id   |  | value  |
+--------+  +--------+
|  0x08  |  |   u8   |
+--------+  +--------+
```
booleans are encoded in 1 byte.

value | encoding |
----- | -------- |
true  | 0x01     |
false | 0x00     |

other values are not allowed.

## string
```rust
"hallo world"
```
strings are UTF-8 encoded sequences of characters.

### value notation
```
str
```
they are 2 types of strings:
- **raw strings:** are strings that correspond to the same text as their content, written with single quotes.
- **escaped strings:** are strings that supports escape sequences, written with double quotes.

for more information, read about [string literals](./object-notation.md#string-literals)

### binary encoding
```
+--------+
|   id   |
+--------+
|  0x20  |
+--------+

base value              value in fields    
+---------+--------+     +-------+     
|   len   | chars  |     | chars |
+---------+--------+     +-------+
| varuint |  len   |     |  len  |
+---------+--------+     +-------+
```
strings are encoded in UTF-8, they consists of a varuint specifing the length in bytes of the string, followed by the content of the string.

in case a string is encoded in a field value, the length section is omitted, and the length is inferred from the `len` section in the field encoding.

## any
```rust
any
```
any is a type that can take any value, it allow schemaless data, even inside schemaed data.

### value notation
```
any_id = "any"
any_value = value
```
```rust
struct MyStruct {
	custom_data: any,
}

MyStruct {
	custom_data: 1 // vuint
}
```

### binary encoding
```
id          value
+--------+  +--------+--------+
|   id   |  |   id   | value  |
+--------+  +--------+--------+
|  0x01  |  | typeid |   N    |
+--------+  +--------+--------+
```
any is encoded through a typeid, followed by a value encoding of that type.