# structom
structom (StructuredAtoms) is a lightweight general data exchange format designed for universal applications, from small human readable object files to large scale data serialization.

structom has 3 different forms for data representation:
- **object notation**: consize human readable systax for data manipulated by humans.
- **binary objects**: effecient direct form for shemaless data.
- **serialized structs**: flattern form for performant data serialization.

structom provide the expected general data types, in addition, it provide additional rich data structures (tagged unions, tupples) and commonly used data types (date, time, color...).

structom supports both schema and schemaless data, and provide support for user defined erased metadata, enhancing the generic data types with specified bases (hexadecimal, utf-16...) and patterns (url, email, uuid...).

structom is designed to be very versatile and expressive, while remaining efficient and performant, adapting for any need from high level rich data notation to low level direct serialization.

# index
### syntax and format
1. [**object notation**](./object-notation.md)
2. [**binary format**](./binary-format.md) 
3. [**declarations**](./declarations.md)

### types and structures
4. [**primitive structures**](./primitive-structures.md)
5. [**primitive types**](./primitive-types.md)
6. [**general data structures**](./general-structures.md)
7. [**rich types**](./rich-types.md)

### other
8. [**standared metadata**](./standared-metadata.md)