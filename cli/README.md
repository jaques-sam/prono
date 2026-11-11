# Prono-cli

- [Prono-cli](#prono-cli)
  - [Summary](#summary)
  - [Build \& Run](#build--run)


## Summary

The cli app is meant for the administrator only.

It has the following functionality:

- surveys (stored in table `Survey`, the backend and the apps get them from there):
  - `surveys list`
  - `surveys upload <file>`: upload a survey definition, see [Adding a questionnaire](../README.md#adding-a-questionnaire);
    it replaces the survey with the same id
  - `surveys scores <survey id>`: the ranking of a scored survey
- users:
  - show all
  - delete one
- access (who may answer a survey, see `"access"` in `api/surveys/*.json`):
  - `access allow <survey id> <first name> <last name>` / `disallow` / `show`: the name list of a
    `"name_list"` survey, stored in table `SurveyAllowedUser`
  - `access pass-phrase <survey id> <phrase>`: the pass phrase of a `"pass_phrase"` survey, stored in
    `SurveyPassPhrase` (the survey stays closed until it is set)

Participants are matched on `<first name> <last name>`, ignoring case.


## Build & Run

```sh
cargo run --bin prono-cli
```

See help with `cargo run --bin prono-cli -- --help`.

It connects to the database with the same [configuration](../backend/README.md#configuration) as the backend.
