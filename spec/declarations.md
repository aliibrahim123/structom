# declarations
declarations are used to define custom data types, and can be used to define schema for the data.

declarations are optional for object notation and binary objects, but are required for serialized structs.

declarations are similar to type definitions in other languages, and allow rich data types like tagged unions and tupples.

declaration files are structom files that contains only declarations with no data.

the only allowed declaration in binary format is the import declaration.

## structures declarations
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
define named types with thier fields, see [primitive structures](./primitive-structures.md).

## import declarations
### object notation
```
"import" str ["as" identifier]
```

### binary encoding
```
+--------+---------+--------+
|  tag   | idspace |  path  |
+--------+---------+--------+
|  0x01  |   u8    | string |
+--------+---------+--------+
```
imports are used to link a declaration file into the scope.

the path can be a relative path, or a url.

in object notation, the content of the declaration file are placed under the specified namespace `ns` declared through `as ns`, else they are reference directly by name.

in binary format, the content of the declaration file are placed under the specified idspace.

the imports only effect the current file.

```javascript
import "./declaration.stom" as my_types

my_types.MyStruct {
	a: 1
}
```

## metadata
```
"@" identifier "(" str ")"
```

metadata are optional erased user defined attributes that are defined on the types identifiers.

they allow to add additional information to the data, without effecting the underlying structure.

metadata are allowed only in object notation, schemaless or in declarations, as they are only usefull for humans.

```rust
struct MyStruct {
	a: @pattern("email") str,
	b: @base("hex") u32
}
```