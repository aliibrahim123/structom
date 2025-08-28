# rich types
```javascript
{
	date: inst "1234-05-06T07:08:09.123Z";
	uuid: uuid "123e4567-e89b-12d3-a456-426655440000";
}
```
rich types are composite types that represent commonly used data types.

the handling of this types are optional, and the parser can return thier raw values.

## time related types
```rust
inst "1234-05-06T07:08:09.123Z"
dur "7h 8m 9s"
```
`inst` and `dur` are composite types that represent datetime values.

`inst` represents an instant in time, it is a `i64` that represents the number of milliseconds passed since the unix epoch (1970-01-01T00:00:00Z) in UTC.

`dur` is a `i64` that represents a duration of time in nanoseconds.

`inst` have a nanosecond precision variant (`instN`).

### instant value notation
```
date_part = (4 * dec_digit | ("+" | "-") dec_digit*) "-" 2 * dec_digit "-" 2 * dec_digit
time_part = 2 * dec_digit ":" 2 * dec_digit ":" 2 * dec_digit ["." dec_digit*]
zone_part = "Z" | ("-" | "+") dec_digit * 2 ":" dec_digit * 2
inst_value = "inst" '"' (date_part | date_part ("T" | " ") time_part zone_part) '"'
```
instant value are written in simplified form of ISO 8601 extended format.

instant value are written according to format `YYYY-MM-DDTHH:MM:SS.ssssZ`:
- `YYYY`: the year part, 4 digits or expanded form (`+` or `-` followed by more than 1 digit).
- `MM`: the month part, 2 digits.
- `DD`: the day part, 2 digits.
- `HH`: the hour part, 2 digits.
- `MM`: the minute part, 2 digits.
- `SS`: the second part, 2 digits.
- `ssss`: the fractional part, 3 to 9 digits.
- `Z`: the timezone part, `Z` for UTC, `+HH:MM` or `-HH:MM` for other offsets.

the time and timezone parts are optional, the `T` separator can be space for convenience.

```javascript
inst "1234-05-06"
inst "-123-04-05 12:34:56Z"
inst "1234-05-06T07:08:09.1234567890+01:30"
```

### duration value notation
```
dur_unit = "ns" | "us" | "ms" | "s" | "m" | "h" | "d" | "mn" | "y"
dur_value = "dur" '"' ["-"] (dic_digit+ dur_unit)+ '"'
```
duration value are written inside a string, they are a series of unsigned integers followed by a time unit.

the numbers must be sorted from largest to smallest unit, and only the largest unit can overflow.

the duration can be negative through a leading `-` character.

suffix | unit        | max value
------ | ----------- | ---------
`y`    | year        | 300
`mn`   | month       | 12
`d`    | day         | 30 (365 if there is no month part)
`h`    | hour        | 24
`m`    | minute      | 60
`s`    | second      | 60
`ms`   | millisecond | 1000
`us`   | microsecond | 1000
`ns`   | nanosecond  | 1000

```javascript
dur "10s"
dur "1m 500ms"
dur "-1y 2mn 3d 4h 5m 6s 7ms 8us 9ns"
```

### binary encoding
```
inst
+--------+  +--------+
|   id   |  | value  |
+--------+  +--------+
|  0x30  |  |  i64   |
+--------+  +--------+

instN       value:
+--------+  +--------+--------+
|   id   |  | off_ms |   ns   |
+--------+  +--------+--------+
|  0x31  |  |  i64   |  u32   |
+--------+  +--------+--------+

dur
+--------+  +--------+
|   id   |  | value  |
+--------+  +--------+
|  0x32  |  |  i64   |
+--------+  +--------+
```
`inst` is encoded through a `i64` number that represents the number of milliseconds passed since the unix epoch.

`instN` is encoded like `inst` but with an extra `u32` that represents the number of nanoseconds in the value.

`dur` is encoded through a `i64` number that represents the number of nanoseconds in the value.

## uuid
```rust
uuid "123e4567-e89b-12d3-a456-426655440000"
```
uuid (universal unique identifier) is a commonly used 128-bit identifier.

### value notation
```
"uuid" '"' 8 * hex_digit "-" 4 * hex_digit "-" 4 * hex_digit "-" 4 * hex_digit "-" 12 * hex_digit '"'
```
`uuid` values are written inside a string, they are a series of 8, 4, 4, then 12 hexadecimal digits separated by hyphens.

### binary encoding
```
+--------+  +--------+
|   id   |  | value  |
+--------+  +--------+
|  0x33  |  |  u128  |
+--------+  +--------+
```
`uuid` is encoded as a `u128` big endian number.