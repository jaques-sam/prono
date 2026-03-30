# Prono Backend

- [Prono Backend](#prono-backend)
  - [Summary](#summary)
  - [Build \& Run](#build--run)
  - [Deployment](#deployment)
    - [Simple testing as NAS user](#simple-testing-as-nas-user)
    - [As a Synology service](#as-a-synology-service)


## Summary

The prono-backend is basically a REST API to access the prono database. It allows all prono-api clients to connect to the prono database.


## Build & Run

Basically, you can build the backend with the following command:

```sh
cargo build --release --bin prono-backend

```
## Deployment

### Simple testing as NAS user

For simple testing,  run the backend with the following command:

```sh
/backend/install/sync.sh  # Syncs the binary to the NAS (in ~)
```
On the NAS:

```sh
RUST_LOG=debug ~/prono-backend
```
Note: you need a config file at `~/.config/prono-backend/config.toml` with the following content:
[Configuration](#../app/README.md#configuration)


### As a Synology service

```sh
/backend/install/build-spk.sh target/release/backend target/  # Using x86 builds
```
Then install the generated `target/prono-backend.spk` file on the NAS using the Synology Package Center.
You can then start the service from the Package Center UI.

This needs an env file at `/var/packages/prono-backend/etc/env` with ENV variables defined in
[configuration](#../app/README.md#configuration) section of the app readme.
