# Prono Backend

- [Prono Backend](#prono-backend)
  - [Summary](#summary)
  - [Security](#security)
    - [API Key Authentication](#api-key-authentication)
    - [Question ID Validation](#question-id-validation)
    - [Device Validation](#device-validation)
  - [Build \& Run](#build--run)
  - [Deployment](#deployment)
    - [Simple testing as NAS user](#simple-testing-as-nas-user)
    - [As a Synology service](#as-a-synology-service)


## Summary

The prono-backend is basically a REST API to access the prono database. It allows all prono-api clients to connect to the prono database.


## Security

The backend implements API key authentication to protect write endpoints:
- **Protected endpoints**: `/api/survey/answer` (POST) - requires API key
- **Public endpoints**: `/api/survey` (GET), `/api/survey/response/*` (GET), `/api/survey/answers/*` (GET)


### API Key Authentication

The API key is baked into both the backend and the client via `prono_api::API_KEY`. Clients include it in the `Authorization` header as `Bearer <key>`.


### Question ID Validation

All answer submissions are validated to ensure the `question_id` exists in the survey definition. Invalid question IDs will be rejected with a 400 Bad Request response.


### Device Validation

When a user is added, a unique device_id is added in the same `Users` table.
This avoids adding the user twice.
This is different from the debug build, in that case it's a random uuid.


## Build & Run

Basically, you can build the backend with the following command:

```sh
cargo build --release --bin prono-backend

```
## Deployment

### Simple testing as NAS user

For simple testing,  run the backend with the following command:

```sh
./backend/install/sync.sh  # Syncs the binary to the NAS (in ~)
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

This needs an env file at `/var/packages/prono-backend/etc/env` with the following environment variables:
- `PRONO_DB_HOST` - Database host
- `PRONO_DB_PORT` - Database port
- `PRONO_DB_USER` - Database username
- `PRONO_DB_PASS` - Database password
