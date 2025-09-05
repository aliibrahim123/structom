# binary format
structom can be encoded in binary format for efficient data serialization.

binary files contains only one root value, they dont support declarations except the import declaration.

binary files are encoded in little endian.

## base encoding
```
+-----------+--------+--------+
| decl_path |   id   | value  |
+-----------+--------+--------+
|    str    | vuint  |   N    |
+-----------+--------+--------+

+--------+----------+
|    0   |   value  |
+--------+----------+
|   u8   | any_type |
+--------+----------+
```
the data starts with a string encoding a declaration file path, followed by a varuint encoding the root value typeid decleared in that file, followed by the root value.

if `decl_path` is an empty string, the root value is of type any.

## fixed size numbers
numbers are encoded in little endian, they can be unsigned or signed encoded in twos complement.

numbers can be 8, 16, 32 or 64 bits.

bits | bytes | signed | unsigned
---- | ----- | ------ | --------
8    | 1     | i8     | u8
16   | 2     | i16    | u16
32   | 4     | i32    | u32
64   | 8     | i64    | u64

## varint and varuint
varints are numbers of variable size encoded in [Little Endian Base 128 (LEB128)](https://en.wikipedia.org/wiki/LEB128), they are signed and unsigned variants. 

each byte encode 7 bit section of the number, from least significant to most significant, with the most significant bit specify if there is another byte to follow.

varints takes 1 to 10 bytes, with values being at most 64 bits.

```
MSB ---------------------------- LSB
 0xxxxxxx
 1xxxxxxx 0xxxxxxx
 1xxxxxxx 1xxxxxxx 0xxxxxxx
 1xxxxxxx 1xxxxxxx 1xxxxxxx 0xxxxxxx
...

   1101110100110010
-> 10110010 10111010 00000011
```

## typeid
```
builtin types
+-+------+
| |  id  |
+-+------+
|0|  u7  |
+-+------+
```
typeids are identifiers used to identify types.

typeids for builtin types are encoded in 7 bit id, with the most significant bit set to 0.