# object notation syntax
structom can be written in human readable object notation, this notation is of c-like syntax like other used formats such as json.

structom file contains one root value with optional declarations.

```js
{
	nb: 1,
	string: "hello",
	map: { key: "val" },
	array: [ 1, 2, 3 ]
}
```

## base syntax
```
identifier = identifierStart identifierPart*
identifierStart = "a" ... "z" | "A" ... "Z" | "_"
identifierPart = identifierStart | "0" ... "9" | "-"

typeid = [identifier "."] identifier

ws = " " | "\t" | '\n' | "\r"

comment = 
	('//' (any_char - '\n')* '\n') |
	('/*' (any_char - '*/')* '*/')
```
**identifiers** are case sensitive names of types, fields...

**typeid** are identifiers used to reference types, it is the type name, with optional namespace at start.

**whitespace** are ignored, they are space, tab, new line and carriage return.

**comments** are ignored, they are written with `//` upto the end of the line or `/* ... */`.
```c
// comment
/* also comment */
```

## number literals
```
dec_digit = "0" ... "9"
dec_part = ("1" ... "9") (dec_digit | "_" dec_digit)*
bin_digit = "0" | "1"
bin_part = bin_digit (bin_digit | "_" bin_digit)*
hex_digit = "0" ... "9" | "a" ... "f" | "A" ... "F"
hex_part = hex_digit (hex_digit | "_" hex_digit)*

number = "0b" bin_part | "0x" hex_part | dec_part
```
**number literals** can be decimal, binary or hexadecimal, optionally separated by underscores.

## string literals
```
any_char = ? any unicode character ?
str_raw = "'" ((any_char - "'") | "''")* "'"
escape_seq = "\" (
	"0" | "n" | "r" | "t" | '"' | "\" 
	| "x" hex_digit hex_digit 
	| "u{" hex_digit 5 * [hex_digit | "_" hex_digit] "}" 
  )
str_escape = '"' (any_char - ('"' | "\") | escape_seq)* '"'
str = str_raw | str_escape
```
strings can be raw or escaped.

**raw strings** are strings that correspond to the same text as their content, they are written with single quotes, single quotes are escaped with 2 single quotes.

**escaped strings** are strings that supports escape sequences, they are written with double quotes.

**escape sequences**:
- `\0`: null character.
- `\n`: new line.
- `\r`: carriage return.
- `\t`: tab.
- `\"`: double quote.
- `\\`: backslash.
- `\xnn`: `\x` followed by two hexadecimal digits, correspond to a byte of `nn` value.
- `\u{...}`: correspond to a unicode code point, `...` is a sequence of up to 6 hexadecimal digits optionally separated by underscores.