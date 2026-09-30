# general structures
```rust
{
	obj: { a: 1, b: "a" },
	array: [1, 2, 3, 4, 5]
}
```
general structures are data structures that can represent any collection of data, sequenced or keyed.

they can have a concrete type, and can accept any type through the `any` type.

## array
```javascript
[1, 2, 3, 4, 5]
```
arrays are ordered collections of values of the same type.

to have an array of different types, use an enum or `any` as the array type.

### object notation
```
array_id = "arr" "<" typeid ">"
array_value = [array_id] "[" ("" | value ("," value)* [","]) "]"
```
array typeid is wriiten by `arr<item_type>`.

array values are written as a list of values separated by commas wrapped inside square brackets, optionally prefixed by the array typeid.

if the type is not inferable and the array has items of different types, it is assumed to be of item type `any`.

```rust
[1, 2, 3] // => varuint[]
arr<u32> [1, 2, 3]
[1, "a", true] // => any[]
```

### binary encoding
```
+--------+--------+
|   id   |  item  |
+--------+--------+
|  0x22  | typeid |
+--------+--------+

base value               value in fields    
+---------+---------+     +-------+
|  count  |  items  |     | items |
+---------+---------+     +-------+
| varuint | len x N |     |  len  |
+---------+---------+     +-------+
```
array typeid is encoded by a byte of value `0x22`, followed by the items typeid.

array values are encoded by a varuint specifing the count of the items, followed by the items values.

in case an array is encoded in a field value, the count section is omitted and the length is infered from the `len` section in the field encoding.

## maps
```rust
{ a: 1, b: 2, c: 3 }
```
maps are collections of keyed values of the same type.

maps can have keys of any non structure type.

to have a map of different types, use an enum or `any` as the map value type.

### object notation
```
map_id = "map" "<" typeid "," typeid ">"
map_item = (identifier | str | "[" value "]") ":" value
map_value = [map_id] "{" ("" | map_item ("," map_item)* [","]) "}"
```
map typeid is wriiten by `map<key_type, value_type>`.

map values are written as a list of key value pairs separated by commas wrapped inside curly brackets, optionally prefixed by the map typeid.

map key can be an identifier, a string, or a value wrapped inside square brackets.

if the type is not inferable and the map has items or keys of different types, it is assumed that these types is `any`.
```rust
{ a: 1, "b": 2 } // => map<str, varuint>
map<u32, str> { [0]: "a", [1]: "b" }
{ [1]: 1, a: 2 } // => map<any, varuint>
```

### binary encoding
```
+--------+--------+--------+
|   id   |  key   | value  |
+--------+--------+--------+
|  0x23  | typeid | typeid |
+--------+--------+--------+

base value                 value in fields    
+---------+----------+     +--------+     
|  count  |  items   |     |  items |
+---------+----------+     +--------+
| varuint | size x N |     |  size  |
+---------+----------+     +--------+

item
+--------+--------+
|  key   | value  |
+--------+--------+
|   N    |   N    |
+--------+--------+
```
map typeid is encoded by a byte of value `0x23`, followed by the keys typeid, then the values typeid.

map values are encoded by a varuint specifing the count of the items, followed by the items values.

map items are encoded as pairs of the key encoding followed by the value encoding.

in case an map is encoded in a field value, the count section is omitted and the size is infered from the `len` section in the field encoding.