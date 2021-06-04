# archbot-rs

![](https://github.com/sksat/archbot-rs/actions/workflows/ci.yml/badge.svg)
![](https://github.com/sksat/archbot-rs/actions/workflows/build-image.yml/badge.svg)
![](https://img.shields.io/docker/image-size/sksat/archbot-rs)

Yet Another [archbot](https://github.com/sfc-arch/archbot) written by Rust.  
This is a Slack bot used in a [**Arch**](https://arch.sfc.wide.ad.jp/) group in Murai lab at SFC.

## Setup & Deploy

```sh
$ git clone https://github.com/sksat/archbot-rs
$ cd archbot-rs
$ cp config-example.toml config.toml
$ nvim config.toml      # add Slack token & members
```

### docker-compose

```sh
$ docker-compose up -d  # use docker image on ghcr.io(build by GitHub Actions)
```

### compose-cd(docker-compose)

[compose-cd](https://github.com/sksat/compose-cd) is Continuous Deployment utility for docker-compose.

```sh
$ wget https://raw.githubusercontent.com/sksat/compose-cd/main/compose-cd
$ ./compose-cd install
$ cd $COMPOSE_CD_SEARCH_ROOT
$ git clone https://github.com/sksat/archbot-rs # use default .compose-cd
```

## Build from source

- Setup Rust toolchain

[rustup](https://rustup.rs) is the recommended installer for Rust toolchain.

You **must** use rustup to install the Rust toolchain even if your OS package includes Rust.
rustup is a well-made installer and is used by most developers.
And the behavior of toolchain installed by the OS package sometimes different from installed by rustup.

```sh
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh    # Install rustup
$ rustup default stable
$ rustup component add clippy
$ source $HOME/.cargo/env   # add this to shell rc file
```

Then, we can use `cargo` command.
This is package manager and build system.

```sh
$ cargo build                           # build(debug mode)
$ cargo run                             # run(debug mode)
$ env RUST_LOG=archbot=debug cargo r    # run(debug mode with debug log)
```

## Author

GitHub: [sksat](https://github.com/sksat)

Twitter: [sksat_tty](https://twitter.com/sksat_tty)
