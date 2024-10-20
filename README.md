# sbe-codegen

**sbe-codegen** is a tool to generate language-specific codecs for data structures encoded using [Simple Binary Encoding](https://github.com/real-logic/simple-binary-encoding).

## Supported languages
- ✅ Rust
- ✅ Python
- ❌ Java/Kotlin

## Installation
sbe-codegen can be installed either by building from sources directly or by downloading from the JFrog crates repository.

### MSRV (Minimum Supported Rust Version)
The MSRV for sbe-codegen is `1.73.0`. 
This means that sbe-codegen will compile with any version of Rust greater than or equal to `1.73.0`.

The code generation is currently broken on the stable channel due to [certain features being unstable](https://github.com/udoprog/genco/issues/39#issuecomment-1737953712).

### Downloading from crates
You can install the latest version of sbe-codegen by running:
```bash
$ cargo install sbe-codegen
```

### Building from sources
Assuming you have this repository checked-out locally, you can build sbe-codegen by running:
```bash
$ cargo build --release
```

## Usage
```bash
$ sbe-codegen --help
SBE multi-language codec generator

Usage: sbe-codegen [OPTIONS] --schema <SCHEMA_PATH> --language <LANGUAGE> --project-name <PROJECT_NAME> --project-path <PROJECT_PATH>

Options:
      --schema <SCHEMA_PATH>         Path to XML SBE schema
      --language <LANGUAGE>          Codec language [possible values: rust, python]
      --project-name <PROJECT_NAME>  Project name
      --project-path <PROJECT_PATH>  Project path
      --project-version <VERSION>    Project version (optional, taken from schema if not specified)
      --with-test-deps               Include test dependencies
      --format                       Format project
  -h, --help                         Print help
```

### Codec generation
The following command generates Rust codecs for the [example schema](./examples/example-schema.xml) in the [`examples`](./examples) directory:
```bash
$ sbe-codegen --schema ./examples/example-schema.xml --language rust --project-name example --project-path ./examples/rust --with-test-deps --format
```

The Python codecs similarly can be generated by running:
```bash
$ sbe-codegen --schema ./examples/example-schema.xml --language python --project-name example --project-path ./examples/python --format
```

### Codec compilation
The Rust codecs then can be compiled by running:
```bash
$ cd ./examples/rust
$ cargo build --release
```

Similarly, the Python codecs can be compiled by running ([maturin](https://github.com/PyO3/maturin) is required):
```bash
$ cd ./examples/python
$ maturin build --release
```

### Example usage of generated codecs
Usage examples of the generated codecs can be found in the tests provided in the [`examples/rust/tests`](./examples/rust/tests) (Rust) or [`examples/python/tests`](./examples/python/tests) (Python) directories.

If you want to test the generated codecs, generate them with the `--with-test-deps` flag and run the tests with:
```bash
$ cd ./examples/rust
$ cargo test
```

```bash
$ cd ./examples/python
$ poetry install
$ maturin dev #--release (for benchmarks)
$ pytest
```
