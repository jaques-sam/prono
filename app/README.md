# Prono App

- [Prono App](#prono-app)
  - [Summary](#summary)
  - [Build \& Run](#build--run)
    - [Web app](#web-app)
    - [Web Deploy](#web-deploy)
    - [Native build](#native-build)
  - [Structure](#structure)


## Summary

Prono is an application to conduct surveys and store the answers in a database.

It's multi-platform (desktop and web) and written in Rust using the [egui](https://github.com/emilk/egui) framework.
Both the desktop and the web app talk to the [Prono Backend](../backend/README.md) over REST; neither needs database credentials.

For now, the web app is deployed as github page: https://jaques-sam.github.io/prono.


## Build & Run

### Web app

You can compile the app to [WASM](https://en.wikipedia.org/wiki/WebAssembly) and publish it as a web page.

[Trunk](https://github.com/trunk-rs) can be used to build for the web target.
1. Install the required target with `rustup target add wasm32-unknown-unknown`.
2. Run `trunk serve --config ./app` to build and serve on `http://127.0.0.1:8080`. Trunk will rebuild automatically if you edit the project.
3. Open `http://127.0.0.1:8080/index.html#dev` in a browser. See the warning below.

> `assets/sw.js` script will try to cache our app, and loads the cached version when it cannot connect to server allowing your app to work offline (like PWA).
> appending `#dev` to `index.html` will skip this caching, allowing us to load the latest builds during development.
> `#dev` also makes the app use a local backend at `http://localhost:8081` instead of the production one.


### Web Deploy

1. Just run `trunk build --release --config ./app`.
2. It will generate a `dist` directory as a "static html" website
3. Upload the `dist` directory to any of the numerous free hosting websites including [GitHub Pages](https://docs.github.com/en/free-pro-team@latest/github/working-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site). The app is auto-deployed to GitHub pages on a push to the `release` branch, see `.github/workflows/pages.yml` for more details.


### Native build

```sh
cargo run --bin prono-app
```
To see more logs, add `RUST_LOG=debug|info` in front.

The app connects to the production backend. Set `PRONO_BACKEND_URL` to use another one, e.g. a local backend:

```sh
PRONO_BACKEND_URL=http://localhost:8081 cargo run --bin prono-app
```

`./local-prod.sh` in the repository root does this for you: it release-builds and starts the latest backend against
the production database (see the [backend configuration](../backend/README.md#configuration)) and runs the app on it.
`./local-prod.sh cli` runs [`prono-cli`](../cli/README.md) against the same database.


## Structure

- `state.rs`: the app's flow (pick a survey, answer, submit, show results) without any drawing; unit tested.
- `ui.rs` and `timeline.rs`: draw the state and turn clicks into actions. No rules live here.
- `client.rs`: the non-blocking REST client ([ehttp](https://github.com/emilk/ehttp)), the same on desktop and web.
- `device.rs`: the random device id the backend binds a participant's name to.

The surveys and answer rules come from the shared [`prono_api`](../api) crate.
