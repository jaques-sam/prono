# Prono-cli

- [Prono-cli](#prono-cli)
  - [Summary](#summary)
  - [Build \& Run](#build--run)


## Summary

The cli app is meant for the administrator only.

Run without a command it opens an interactive menu (arrows and enter to choose, Esc to go back), for everything below.
With a command it does just that, for scripts.

It has the following functionality:

- surveys (stored in table `Survey`, the backend and the apps get them from there):
  - `surveys list`
  - `surveys upload <file>`: upload a survey definition, see [Adding a questionnaire](../README.md#adding-a-questionnaire);
    it replaces the survey with the same id
  - `surveys pass-phrase <survey id> <phrase>`: the pass phrase that opens a survey, stored in `SurveyPassPhrase`
    (the survey stays closed until it is set; surveys with the same pass phrase are shown together)
  - `surveys scores <survey id>`: the ranking of a scored survey
- users:
  - show all
  - delete one


## Build & Run

```sh
cargo run --bin prono-cli
```

See help with `cargo run --bin prono-cli -- --help`.

It connects to the database with the same [configuration](../backend/README.md#configuration) as the backend.
