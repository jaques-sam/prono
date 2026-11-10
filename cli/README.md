# Prono-cli

- [Prono-cli](#prono-cli)
  - [Summary](#summary)
  - [Build \& Run](#build--run)


## Summary

The cli app is meant for the administrator only.

It has the following functionality:

- users:
  - show all
  - delete one


## Build & Run

```sh
cargo run --bin prono-cli
```

See help with `cargo run --bin prono-cli -- --help`.

It connects to the database with the same [configuration](../backend/README.md#configuration) as the backend.
