# Contribution

## Requirements

- Install rust 1.93 or higher: https://rust-lang.org/learn/get-started/
- Install cargo-binstall: https://github.com/cargo-bins/cargo-binstall
- Install cargo tools using binstall:
```sh
cargo binstall cargo-audit cargo-tarpaulin cargo typos-cli lychee trunk
```

On Linux you need the following libraries:

`sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev`

On Fedora Rawhide you need to run:

`sudo dnf install clang clang-devel clang-tools-extra libxkbcommon-devel pkg-config openssl-devel libxcb-devel gtk3-devel atk fontconfig-devel`


## Validation

1. Run `./checks.sh` to validate your changes
2. Push and open a PR with default target branch `main`

It will be automatically deployed if the `release` branch is updated.
See `.github/workflows/pages.yml` for more details.

Made possible by [this template repo](https://github.com/emilk/eframe_template) for [eframe](https://github.com/emilk/egui/tree/master/crates/eframe).


# Development

## About egui

The official egui docs are at <https://docs.rs/egui>. If you prefer watching a video introduction, check out <https://www.youtube.com/watch?v=NtUkr_z7l84>. For inspiration, check out the [the egui web demo](https://emilk.github.io/egui/index.html) and follow the links in it to its source code.


## Testing locally

- The prono-app can run natively on your computer, or be compiled to WebAssembly and run in a web browser.
- The prono-cli tool can only run natively.
- The prono-backend


### Native run

The app: `RUST_LOG=debug cargo run --release --bin prono-app`
The cli tool: `RUST_LOG=debug cargo run --release --bin prono-cli`
The backend: `RUST_LOG=debug cargo run --release --bin prono-backend`


### Web Locally

You can compile your app to [WASM](https://en.wikipedia.org/wiki/WebAssembly) and publish it as a web page.

We use [Trunk](https://github.com/trunk-rs) to build for web target.
1. Install the required target with `rustup target add wasm32-unknown-unknown`.
2. Run `trunk serve` to build and serve on `http://127.0.0.1:8080`. Trunk will rebuild automatically if you edit the project.
3. Open `http://127.0.0.1:8080/index.html#dev` in a browser. See the warning below.

> `assets/sw.js` script will try to cache our app, and loads the cached version when it cannot connect to server allowing your app to work offline (like PWA).
> appending `#dev` to `index.html` will skip this caching, allowing us to load the latest builds during development.


### Web Deploy

1. Just run `trunk build --release`.
2. It will generate a `dist` directory as a "static html" website
3. Upload the `dist` directory to any of the numerous free hosting websites including [GitHub Pages](https://docs.github.com/en/free-pro-team@latest/github/working-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site). The app is auto-deployed to GitHub pages on a github release, see `.github/workflows/pages.yml` for more details (there is also an option to auto-deploy on push to `master` branch).


## Updating egui

As of 2023, egui is in active development with frequent releases with breaking changes.
When updating `egui` and `eframe` it is recommended you do so one version at the time, and read about the changes in [the egui changelog](https://github.com/emilk/egui/blob/master/CHANGELOG.md) and [eframe changelog](https://github.com/emilk/egui/blob/master/crates/eframe/CHANGELOG.md).
