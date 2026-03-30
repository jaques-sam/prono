# Prono App

- [Prono App](#prono-app)
  - [Summary](#summary)
  - [Build \& Run](#build--run)
    - [Web app](#web-app)
    - [Web Deploy](#web-deploy)
    - [Native build](#native-build)
      - [Configuration](#configuration)
        - [Feed into the Prono App](#feed-into-the-prono-app)


## Summary

Prono is an application to conduct surveys and store the answers in a database.

It's multi-platform (desktop and web) and written in Rust using the [egui](ehttps://github.com/emilk/egui) framework.

For now, the web app is deployed as github page: https://jaques-sam.github.io/prono.

To run the desktop app, see instructions below.


## Build & Run

### Web app

You can compile the app to [WASM](https://en.wikipedia.org/wiki/WebAssembly) and publish it as a web page.

[Trunk](https://github.com/trunk-rs) can be used to build for the web target.
1. Install the required target with `rustup target add wasm32-unknown-unknown`.
2. Run `trunk serve --config ./app` to build and serve on `http://127.0.0.1:8080`. Trunk will rebuild automatically if you edit the project.
3. Open `http://127.0.0.1:8080/index.html#dev` in a browser. See the warning below.

> `assets/sw.js` script will try to cache our app, and loads the cached version when it cannot connect to server allowing your app to work offline (like PWA).
> appending `#dev` to `index.html` will skip this caching, allowing us to load the latest builds during development.


### Web Deploy

1. Just run `trunk build --release`.
2. It will generate a `dist` directory as a "static html" website
3. Upload the `dist` directory to any of the numerous free hosting websites including [GitHub Pages](https://docs.github.com/en/free-pro-team@latest/github/working-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site). The app is auto-deployed to GitHub pages on a github release, see `.github/workflows/pages.yml` for more details (there is also an option to auto-deploy on push to `master` branch).


### Native build

```sh
cargo run --bin prono-app
```
To see more logs, add `RUST_LOG=debug|info` in front.

The debug build has some more functions than the release build.

- When voted, the user can add a new vote using a different user
- If the database connection fails, it will use the fake db with fake data

They can be found by searching for the `debug_assertions` attribute.


#### Configuration

The configuration is only needed/used in the native app, to connect to the prono database.

Parameters:

- host: hostname or IP address of the database server
- port: port number (16 bit) of the database server
- user: username to connect to the database
- pass: password to connect to the database


##### Feed into the Prono App

Either provide a `config.toml` file in the default configuration location:
- MacOS: `$HOME/Library/Application Support/prono/`
- Linux: `$HOME/.config/prono/`
- Windows: `C:\Users\Administrator\AppData\Roaming\prono`

with the following contents:

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
