# binary format
structom can be encoded in binary format for efficient data serialization.

binary files contains only one root value, they dont support declarations except the import declaration.

binary files are encoded in little endian.

## base encoding
```
+------------+--------+-----------+--------+
| decl_count | decls  | root_type | value  |
+------------+--------+-----------+--------+
|    u8      |   N    |  typeid   |   N    |
+------------+--------+-----------+--------+
```
the data starts with the number of declarations, followed by the declarations (if there), and finally the root typeid then its value.

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

each byte encode 7 bit section of the number, from least significant to most significant, with the most significant bit specify if there is another byte to follow

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

user defined types
+-+---------+---------+
| | idspace | |  id   |
+-+---------+---------+
|1|   u7    |0|  u7   |
+-+---------+---------+

+-+---------+---------+
| | idspace | |  id   |
+-+---------+---------+
|1|   u7    |1|  u15  |
+-+---------+---------+
```
typeids are identifiers used to identify types.

typeids for builtin types are encoded in 7 bit id.

typeids for user defined types are encoded in 7 bit idspace defined by the import declaration, and a u7 or u15 id to the type as defined in the declaration file.