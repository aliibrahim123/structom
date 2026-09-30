code generation for structom serialization.

generate serialization code for every decleration file in a given directory in a given language.

**avaialible languages**: rust, javascript.

```
Usage: structom-codegen --input <INPUT> --output <OUTPUT> --lang <LANG>

Options:
  -i, --input <INPUT>    declerations directory path
  -o, --output <OUTPUT>  generated code output path
  -l, --lang <LANG>      language of the generated code [possible values: rust, js]
  -h, --help             Print help
  -V, --version          Print version
```