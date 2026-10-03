# Structom

structom, structured atoms, is a general purpose data exchange format designed for universal applications.

structom achieve its universality by providing rich and expressive data representation constructs that are highly efficient, lightweight, and bidirectionly compatible.

this document serves as a language specification for structom, documenting all its featureset from data layout till rich types.

### feature set

- expressive object notation designed to ergonomic human readable data files.
- efficient binary encoding desgined for bare metal performance data serialization.
- rich schema ability with module system and forward and backward compatibility.
- schemaless version with full featureset support in object notation and binary encoding.
- rich modeling contructs: tagged unions, tagged types, tree structs, generics, bitfields...
- zero copy binary decoding with streaming and out of order decoding support.
- wide set of rich data types: datetime, duration, uuid, url...
- erasable metadata support for better tooling with common builtin ones.
- simple core with easily ignoreable implementation selected optional extentions.

# foundations

## object notation

the object notation is json like human readable format of structom, it is designed for expressive data files edited by humans.

```structom
import "./schema.stomd"
Root {
	// comment
	a: "hello world",
	#metadata
	b: 123.456_789,
	c: [true, Color.HSL(1, 2, 3), 0x"ab cd ff"]
}
```

the [gramex](https://docs.rs/gramex/latest/gramex/gram_ref/) metasyntax is used here in syntax definition.

the object notation uses the utf-8 encoding, and the first [`BYTE ORDER MARK (U+FEFF)`](https://en.wikipedia.org/wiki/Byte_order_mark#UTF-8) is removed.

whitespace is insignificant and ignored, used only in token separation.

```gramex
let ws = ' ' | '\t' | '\n' | '\r';
```

identifiers are case sensitive names of things: types, fields, namespaces, variants, and metadata.

```gramex
let ident_start = 'a'..'z' | 'A'..'Z' | '_';
let ident = ident_start (ident_start | '0'..'9')*;
```

`list` is a grammar construct that specifies a `,` separated list of items, with optional trailing `,`.

```gramex
let list<item> = item (',' item)* ','?;
```

### comments

comments follow the general c style of line (`//`) and block (`/* ... */`) comments. block comments can nest.

in addition, unit comments (`/-`) comment out an the entire items they prefix, they are declerations (struct, enums, bitfields), imports, metadata, types and values.

in addition, they comments out array items, map items, fields, tuple fields and generic params, alongside the `,` after them if found.

#### example

```gramex
let line_comment = "//" !'\n'* '\n'?;
let block_comment = "/*" (block_comment | !"*/")* "*/";
let unit_comments = "/-" /* context defined unit */;
```

```structom
/* block comment
	/* nested comment */
*/
struct Struct {
	// line comment
	/- field1: nb,
	field1: str,
}
```

### root structure

an object file starts with zero or more imports, then optionally local item declerations, and lastly a root value.

```gramex
let file = import* item* root_value:value;
```

#### example

```structom
#!version("0.1.2")

import "./schema.stomd"
import "../extensions.stomd" as ext

struct Tagged (u32)

Root {
	a: 123,
	b: "hallo",
	c: Tagged 3
}
```

## binary encoding

the binary encoding is machine friendly format of structom designed for high performance serialization

a binary file is interpreted as a sequence of bytes, where data is layed in nested fields.

data is encoded in little endian format aligned to their natural alignment, with some additional padding between fields.

### core field types

data is encoded as signed / unsigned integers of `n` bits, specified through `u/in` field type.

array like data is encoded as a field encoding the length, followed by the items fields, they are always `0`-indexed.

pointers are `i32` fields encoding byte offsets relative to the pointer position, pointing to fields in the curent section.

#### example

![binary example](./binary_example.svg)

### header

![header](./header.svg)

a binary file starts a header specifing core attributes, it is required to standalone files but optional for inlined blobs.

the header begins with the magic number `53 54 4F 4D (STOM)`.

then it followed by a list of imports encoded as `arr<str>`, it starts with a `u32` length field followed by `str`s encoding the imports paths.

then a list of sections encoded as `arr<u64>`, it starts with a non zero `u32` length field followed by `u64` section offsets.

if there is one section, no offsets need to be specified.

then a `typeid` of the root value is followed.

#### example

![header example](./header_example.svg)

header of 1 import (`"schema"`), 1 section and root value `import("schema").first_type`;

### sections

![sections](./sections.svg)

sections are non uniformly sized part of a binary file.

they created a 2 level address space for the file, decreasing the pointer length (`u32`) while supporting `u64` file sizes.

sections are composed of multiple objects, each object encoded a specific data structure defined by the schema.

objects are aligned to `u64` boundries, they starts with a `u32` field encoding the full size of the object.

the root value is the first object in the first section, all other objects decend from it.

# declerations

declerations are object notation constructs that acts as schemas and define the shape and of data structures.

they are declared above the root value in object files, or in their own specific file called decleration files.

local declerations can refer to each other in any order, not necessary items only above them.

each item gets its own `0` indexed id based on its order in file, with max of `65536 (2 ^ 16)` items inside a single file.

#### example

```structom
import "./module1.stomd"

struct Root {
	a: u8,
	b: Enum
	c: bitfield(u8) {
		d: b3,
		e: b5,
	},
}

enum Enum {
	A, B, C,
	D(u64),
	E { v?: arr<uuid> },
}
```

## imports

```gramex
let import = "import" path:str ("as" ns:ident)?;
```

imports bring the declerations of another decleration file into the scope.

they are declared by the `import` keyword followed by a `str` representing the url / fs path to the decleration file.

if a `ns` identifier is specified, the imported declerations are bringed in their own namespace.

imports can not be circular.

#### example

```structom
// module1.stomd
struct Struct { }

// module2.stomd
import "./module1.stomd"
enum Enum { }

// module3.stomd
import "./module1.stomd"
import "./module2.stomd" as mod2

sturct Root {
	a: Struct,
	b: mod2.Enum
}
```

---

in binary encoding, imports are encoded as `arr<str>` inside the header, where the namespace id is the import index in the array + 1.

## type

### type specifier

```gramex
let type_spec = (ns:ident '.')? item:ident ('<' args:list<type_spec> '>')?;
```

type specifiers are object notation contructs that resolves to types, used where a type is expected.

in its basic form, it is the identifier to the type, resolved in the current scope.

a namespace identifier can be specified at the begining, the type is then resolve from that namespace.

if the type is generic, its arguments are specified as type specifiers inside a `,` sperated list enclosed in angle brakets.

#### example

```structom
import "./module1.stomd" as mod1.
struct Local { }
enum Generic<A, B, C> { }

struct Root {
	a: Local,
	b: mod.External,
	b: Generic<u8, str, arr<f64>>
}
```

### type identifiers

![type identifier](./type_id.svg)

type identifiers are fields in binary encoding that specify the type of a value.

they are composed of 2 `u16` fields, a `ns` namespace identifer, and the type `id` in the namespace.

`ns` `0` is the builtin types namespace, while the rest are assigned to the imported files as their index in `import` array in the header + 1.

#### example

![typeid example](./typeid_example.svg)

```sturctom
/// module1.stomd
struct Struct { }
Enum { A }

/// value.stomo
import "./module1.stomd"
[u8 10, Enum.A]
```

## metadata

```gramex
let metadata = '#' '!'? name:ident ('(' args:list<value> ')')?;
```

metadata is object notation specific contruct for user defined extra data.

they are usefull for outside tooling and ignored by the parser.

metadata is declared before items, values, fields and variants, using a `#` followed by the metadata name.

metadata can optionally have a value arguments enclosed in paranthesis.

metadata for the whole file are declared before any items and using `#!name`.

#### example

```structom
#!version("0.1.2")

#doc("user info")
struct User {
	#pattern("[a-zA-Z0-9]+")
	name: str,
	id: #private_constructor
		sturct UserId(uuid),
}
```

# core types

# base types

# structs

# union

# collections

# rich types

# apendix
