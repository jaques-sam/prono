# Prono

[![dependency status](https://deps.rs/repo/github/jaques-sam/prono/status.svg)](https://deps.rs/repo/github/jaques-sam/prono)
[![Build Status](https://github.com/jaques-sam/prono/workflows/CI/badge.svg)](https://github.com/jaques-sam/prono/actions?workflow=CI)

- [Prono](#prono)
  - [Summary](#summary)
  - [Architecture](#architecture)
    - [Adding a questionnaire](#adding-a-questionnaire)
  - [Contents](#contents)


## Summary

This repository is a combination of applications that interact to

  - conduct surveys,
  - collect them in a database
  - and visualize the results.


## Architecture

```mermaid
flowchart TB
    web["web app (wasm)"] -- REST --> backend
    native["native app"] -- REST --> backend
    subgraph backend [prono-backend]
        rest["REST endpoints"] --> core["prono: submit / results, Repo"]
    end
    cli["prono-cli"] --> core
    core -- sqlx --> db[(MariaDB)]
    api["prono_api: surveys, answers, rules, REST contract"]
    web & native & backend & cli -.-> api
```

| Crate | Role |
| --- | --- |
| `api` (`prono_api`) | The model every binary shares: the survey catalog (`api/surveys/*.json`), `Answer`, the validation rules and the REST contract. Compiles to wasm. |
| `prono` | Server-side core: the `submit` and `results` use cases, the `Repo` trait with its MySQL and in-memory implementations, and the database config. |
| `backend` | `actix-web` REST server around `prono`. |
| `app` | egui app for desktop and web. Both talk to the backend over REST; the UI only draws a small state machine (`app/src/state.rs`). |
| `cli` | Admin tool on top of `prono`. |

There is one definition of each survey type (in `prono_api`), and the rules
live there too, so the app checks a submission with exactly the code the
backend uses to accept it.

> The diagram in [`docs/architecture.drawio.png`](docs/architecture.drawio.png) predates this layout.


### Adding a questionnaire

1. Add a JSON file to `api/surveys/` with a new, unique `survey_id`:
   ```json
   {
       "survey_id": 2,
       "description": "My survey",
       "link": "https://example.com",
       "questions": [
           { "question": "When will it happen?", "answer_type": "prediction_date" },
           { "question": "Why?", "answer_type": "text" }
       ]
   }
   ```
2. List it in `SURVEY_FILES` in `api/src/catalog.rs`.
3. Redeploy the backend and the apps.

A question's id is derived from its text and answer type, and stored answers
reference that id: editing a question's text starts it from scratch.


## Contents

This repository contains the following applications:

- [Prono App](app/README.md)
- [Prono Backend](backend/README.md)
- [Prono CLI](cli/README.md)



