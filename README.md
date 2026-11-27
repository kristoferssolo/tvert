# tvert

A Wayland-native screenshot tool for Linux, built in Rust.

## Installation

```sh
cargo install --locked --git https://github.com/kristoferssolo/tvert tvert-cli
```

Prebuilt Linux binaries are attached to each
[release](https://github.com/kristoferssolo/tvert/releases).

## Usage

```sh
tvert preview region               # select a region and open the editor
tvert preview screen --output DP-1 # capture one monitor
tvert save region --path shot.png  # select a region and save it immediately
tvert config print-default         # print the default configuration
```

Configuration is read from `$XDG_CONFIG_HOME/tvert/config.toml`, or
`~/.config/tvert/config.toml`. Pass `--config FILE` to use another file.

## Development

```sh
just setup  # install the Rust toolchain and the tools pinned in mise.toml
just check  # formatting, Clippy, tests, and docs, as CI runs them
just        # list all recipes
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
