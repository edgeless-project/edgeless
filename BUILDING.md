# EDGELESS building instructions

The implementation relies on [Rust](https://www.rust-lang.org/).
[gRPC](https://grpc.io/) and [protobuf](https://protobuf.dev/) are used for
interprocess communication.

Once you have a working environment, building the core components and tools is done simply with the following command:

```bash
cargo build
```

Below you will find instructions to create different flavors of your working environment.

- [EDGELESS building instructions](#edgeless-building-instructions)
  - [Ubuntu 22.04/24.04](#ubuntu-22042404)
  - [Devcontainer](#devcontainer)
  - [NixOS](#nixos)
  - [Cross-compiling for aarch64](#cross-compiling-for-aarch64)
  - [Mac OS](#mac-os)

## Ubuntu 22.04/24.04

Install the dependencies:

```bash
source "$HOME/.cargo/env"
sudo apt update && sudo apt install curl git gcc libssl-dev pkg-config unzip make g++ libtss2-dev -y
```

Install Rust (follow the interactive instructions):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Install WASM target in the Rust toolchain and the wasm-opt utility, which is
needed by `edgeless_cli function build`:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-opt
```

If using Ubuntu 24.04 or newer, you can install protobuf with apt:

```bash
sudo apt update && sudo apt install protobuf-compiler libprotobuf-dev -y
```

Otherwise, with Ubuntu 22.04 or older, you must to install a modern release of the Protocol Buffers binaries
(v31.1 is the latest stable release as of 16/06/2025).
The available binaries in the default Ubuntu 22.04 repositories have proven to be too old for some EDGELESS crates.

```bash
# Get latest TAG
PROTOC_VERSION=$(curl -s "https://api.github.com/repos/protocolbuffers/protobuf/releases/latest" | grep -Po '"tag_name": "v\K[0-9.]+')
wget -qO protoc.zip https://github.com/protocolbuffers/protobuf/releases/latest/download/protoc-${PROTOC_VERSION}-linux-x86_64.zip
sudo unzip -q protoc.zip bin/include -d /usr/local
sudo unzip -q protoc.zip bin/protoc -d /usr/local
sudo chmod a+x /usr/local/bin/protoc
rm -rf protoc.zip
```

At this point you may have to logout/login to let your shell know of the new
executable, just try `protoc --version` to see if it works.

Finally clone the repo and have fun with EDGELESS:

```bash
git clone https://github.com/edgeless-project/edgeless.git
cd edgeless
```

To build the debug executables:

```bash
cargo build
```

To build the release executables:

```bash
cargo build --release
```

To run the tests:

```bash
cargo test
```

## Devcontainer

An easy and clean way to get started is the `devcontainer` shipped as part of this
repository. 

1. It makes sense to clone the repository directly into a `devcontainer` to avoid
bind mounts and possibly make builds faster. To do this, install VSCode, and
select: `DevContainers: Clone Repository in Named Container Volume`. It should
prompt you to a GitHub page in your browser where you can authenticate. On an
M1 Max, the achieved speedup was around x10 for `cargo build`.

2. There is a script to configure some plugins for `zsh`:
`scripts/enhance_zsh_dev_container.sh`, which is entirely optional. After
running it things like autocompletion and shell syntax highlighting are
available. Feel free to modify it to your liking!

3. If your build times are still horrible, try to allocate more CPUs and RAM to
   the Docker dev_container.


## NixOS

If using Nix / on NixOS then there is a simple [`flake.nix`](./flake.nix) that is invoked via the `direnv` [`.envrc`](./.envrc) to autoinstall Nix package dependencies and give you a bulid shell once you `direnv allow` in this directory.

To build the function examples under `./examples` you will need to add the WASM toolchain via `rustup`:

```shell
rustup target add wasm32-unknown-unknown
```

## Cross-compiling for aarch64

Follow the same instructions for Ubuntu above, then add the `aarch64` target
to Rust:

```bash
rustup target add aarch64-unknown-linux-gnu
```

When building, specify that target, e.g., to build the release version
from the `edgeless` root:

```bash
cargo build --release --target aarch64-unknown-linux-gnu
```

## Mac OS

Tested on Apple M3 Pro with Sonoma 14.2.1

```bash
brew install protobuf
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup target add wasm32-unknown-unknown
cargo install wasm-opt
git clone https://github.com/edgeless-project/edgeless.git
cd edgeless
cargo build
```
