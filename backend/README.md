# Prono Backend

- [Prono Backend](#prono-backend)
  - [Summary](#summary)
  - [REST API](#rest-api)
  - [Rules](#rules)
  - [Configuration](#configuration)
  - [Database](#database)
  - [Build \& Run](#build--run)
  - [Deployment](#deployment)
    - [Simple testing as NAS user](#simple-testing-as-nas-user)
    - [As a Synology service](#as-a-synology-service)


## Summary

The prono-backend is a REST API in front of the prono database. Both the web and the desktop app use it.


## REST API

| Method & path | Body | Response |
| --- | --- | --- |
| `POST /api/surveys/{survey_id}/answers` | `{"user": "...", "answers": [["<question id>", <answer>], ...]}` | `200`, or an error |
| `POST /api/surveys` | `"the pass phrase"`, or `null` for none | The surveys that pass phrase opens, a JSON list in survey id order; `403` with `"Wrong pass phrase"` when it opens none. This is the only way the apps get surveys |
| `GET /api/surveys/{survey_id}/answers` | | `{"<question id>": [{"user": "...", "answer": <answer>}]}` |

An `<answer>` is `{"PredictionDate": {"day": null, "month": 3, "year": 2026}}` or `{"Text": "..."}`.

`POST` needs two headers:
- `Authorization: Bearer <prono_api::API_KEY>`. The key is baked into the apps, including the public web app, so it only keeps out casual scripts.
- `X-Device-Id: <device id>`, generated and stored by the app.

Errors come back as a JSON string meant for the participant: `400` for an invalid submission, `401` for a bad API key, `403` for a device conflict, `409` when the survey was answered before, `500` otherwise (details only in the log).


## Rules

Enforced in `prono::submit`, with the checks shared with the apps (`prono_api::Submission::validate`):

- A submission answers every question of the survey exactly once, with a valid answer.
- A name is bound to the device that first submitted for it.
- A device can only be used by one participant.
- A participant answers a survey once. All answers of a submission are stored in one transaction.


## Configuration

The backend (and [the CLI](../cli/README.md)) read the database settings from `secure_config.toml` in the configuration directory:
- MacOS: `$HOME/Library/Application Support/prono/`
- Linux: `$HOME/.config/prono/`
- Windows: `C:\Users\<user>\AppData\Roaming\prono`

```toml
[db]
host = "the_prono_db_host"
port = 3306
user = "the_prono_db_user"
pass = "the_prono_db_password"
```

Each setting can be overridden (or provided without a file) through `PRONO_DB_HOST`, `PRONO_DB_PORT`, `PRONO_DB_USER` and `PRONO_DB_PASS`.


## Database

A MariaDB/MySQL database named `db_prono`. Log in with:

```bash
mysql db_prono -vh ${HOST} -P ${PORT} -u ${USER} -p --connect-timeout=2
```

and create the tables once with `install/schema.sql`. For the NAS, `install/create_db.sh` does everything
(database, tables and the app user from your `secure_config.toml`) over `ssh nas`; it asks for the MariaDB root password.

> Synology's MariaDB 10 starts with `skip_networking=1`: enable *TCP/IP connection* in the MariaDB 10 package settings
> (DSM) before the backend or the CLI can reach it over `host`/`port`.
>
> MariaDB there has no SSL (`have_ssl = DISABLED`), so keep the database traffic on the NAS and the LAN: the app user can
> only log in from `localhost`, `127.0.0.1` and `192.168.80.%` (see `HOSTS` in `create_db.sh`, never `%`), the backend uses
> `host = "127.0.0.1"`, and the CLI from another machine uses the LAN or an SSH tunnel (`ssh -L 3306:127.0.0.1:3306 nas`,
> then `host = "127.0.0.1"`). `create_db.sh` also resets the app user's password to the one in `secure_config.toml`.

Question ids are UUIDs, stored in `AnswerResponse.question_id` as `BINARY(16)` (`HEX(question_id)` shows them in a `mysql` session).
The surveys themselves are stored in `Survey` (their JSON definitions), uploaded with `prono-cli surveys upload`: the
backend has none until one is uploaded. Every survey is opened by its pass phrase, stored in `SurveyPassPhrase` and
set with `prono-cli surveys pass-phrase`: a participant only gets the surveys whose pass phrase they enter, and a
survey without one stays closed. Run `install/schema.sql` (or `create_db.sh`) again when upgrading an existing
database, **before** the new backend starts: it is idempotent, and it strips the `"access"` key that older surveys
were uploaded with (a definition may not have it any more). The `SurveyAllowedUser` table holds the name lists of the
surveys with `"name_list": true` (set with `prono-cli surveys names`): besides the pass phrase, a submission is only
accepted from a listed name, else `403` with `"You are not on the list for this survey"`. Names left in that table by
an earlier version count again as soon as the survey says `"name_list": true`. The script
also converts a database and its tables to `utf8mb4`, which the
emoji in surveys and answers need (`utf8mb3` fails with `Incorrect string value`); this needs a user with `ALTER`
rights, so run it as root, like `create_db.sh` does.
Answers are stored as text: `DD/MM/YYYY` for dates (`00` when there is no day), the text itself otherwise.


## Build & Run

```sh
cargo build --release --bin prono-backend
```

The queries are checked at runtime, so building does not need a database.


## Deployment

### Simple testing as NAS user

For simple testing, run the backend with the following command:

```sh
./backend/install/sync.sh  # Syncs the binary to the NAS (in ~)
```
On the NAS:

```sh
RUST_LOG=debug ~/prono-backend
```
This needs the [configuration](#configuration).


### As a Synology service

```sh
./backend/install/build-spk.sh  # Packages target/release/prono-backend into target/release/ (x86 builds)
```
Then install the generated `target/prono-backend.spk` file on the NAS using the Synology Package Center.
You can then start the service from the Package Center UI.

This needs an env file at `/var/packages/prono-backend/etc/env` with the `PRONO_DB_*` environment variables from the [configuration](#configuration).
