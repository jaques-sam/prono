# Contribution

- [Contribution](#contribution)
  - [Requirements](#requirements)
  - [Validation](#validation)
- [Development](#development)
  - [About egui](#about-egui)
    - [Updating egui](#updating-egui)
  - [Testing locally](#testing-locally)
    - [Native run](#native-run)
    - [Web Locally](#web-locally)


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


### Updating egui

As of 2023, egui is in active development with frequent releases with breaking changes.
When updating `egui` and `eframe` it is recommended you do so one version at the time, and read about the changes in [the egui changelog](https://github.com/emilk/egui/blob/master/CHANGELOG.md) and [eframe changelog](https://github.com/emilk/egui/blob/master/crates/eframe/CHANGELOG.md).


## Testing locally

- The prono-app can run natively on your computer, or be compiled to WebAssembly and run in a web browser.
- The prono-cli tool can only run natively.
- The prono-backend


### Native run

The app: [see here](app/README.md#native-build)
The cli tool: [see here](cli/README.md#build--run)
The backend: [see here](backend/README.md#build--run)


### Web Locally

Only the app can be compiled to WebAssembly and run in a web browser. See [here](app/README.md#web-app) for instructions.
