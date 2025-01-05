# Rust Coverage Generator

This crate contains a single executable to generate coverage reports.

The coverage generator can generate 2 different reports:
1. a nice HTML page with an overview of the full coverage
2. inline code coverage for developers visible by Coverage Gutters

This runs all unit tests using cargo nextest.

## Table of contents

- [Rust Coverage Generator](#rust-coverage-generator)
  - [Table of contents](#table-of-contents)
  - [Coverage reports](#coverage-reports)
    - [HTML Page](#html-page)
    - [Inline code coverage](#inline-code-coverage)


## Coverage reports

### HTML Page

Build a coverage report as html page by running the `coverage_generator` binary.
The starting point to access the webpage is: `./target/coverage/html/index.html`

```sh
cargo run --bin coverage_generator
```

### Inline code coverage

To have inline code coverage with lcov, add the `--dev-mode` option:

```sh
cargo run --bin coverage_generator -- --dev-mode
```
**Note:** Additional arguments (after `--` are passed to `cargo nextest`)

The lcov file is generated in `./target/coverage/tests.lcov` and can be used by CoverageGutters.
This is already configured for VScode.
