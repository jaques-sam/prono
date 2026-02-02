# Prono App

[![dependency status](https://deps.rs/repo/github/jaques-sam/prono/status.svg)](https://deps.rs/repo/github/jaques-sam/prono)
[![Build Status](https://github.com/jaques-sam/prono/workflows/CI/badge.svg)](https://github.com/jaques-sam/prono/actions?workflow=CI)

- [Prono App](#prono-app)
  - [Summary](#summary)
  - [Building and Running](#building-and-running)
  - [Configuration](#configuration)
    - [Parameters](#parameters)
    - [Feed into the Prono App](#feed-into-the-prono-app)


## Summary

Prono is an application to conduct surveys and store the answers in a database.

It's multi-platform (desktop and web) and written in Rust using the [egui](ehttps://github.com/emilk/egui) framework.

For now, the web app is deployed as github page: https://jaques-sam.github.io/prono/ [INCOMPLETE].

To run the desktop app, see instructions below.

## Building and Running

```sh
cargo run --bin prono-app
```
To see more logs, add `RUST_LOG=debug|info` in front.


## Configuration

### Parameters

Needed for the Prono database:

- host: hostname or IP address of the database server
- port: port number (16 bit) of the database server
- user: username to connect to the database
- pass: password to connect to the database


### Feed into the Prono App

Either provide a `config.toml` file in `$HOME/.config/prono/` with the following contents:

```toml
[db]
host = "the_prono_db_host"
port = the_prono_db_port
user = "the_prono_db_user"
pass = "the_prono_db_password"
```

or set the following environment variables:

- `PRONO_DB_HOST`
- `PRONO_DB_PORT`
- `PRONO_DB_USER`
- `PRONO_DB_PASS`
