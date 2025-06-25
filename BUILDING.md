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
  - [Jetson Nano/Ubuntu 18.04](#jetson-nanoubuntu-1804)
  - [Mac OS](#mac-os)

## Ubuntu 22.04/24.04

Install Rust (follow the interactive instructions):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Install the dependencies:

```bash
source "$HOME/.cargo/env"
sudo apt update && sudo apt install curl git gcc libssl-dev pkg-config unzip make g++ -y
rustup target add wasm32-unknown-unknown
cargo install wasm-opt
```

Install a modern release of the Protocol Buffers binaries (v28.2 is the latest
stable release as of 22/10/2024)
The available binaries in the default ubuntu repositories have proven to be too
old for some EDGELESS crates.

```bash
wget https://github.com/protocolbuffers/protobuf/releases/download/v28.2/protoc-28.2-linux-x86_64.zip
cd /usr/local
unzip $OLDPWD/protoc-28.2-linux-x86_64.zip
rm -f readme.txt
cd -
rm protoc-28.2-linux-x86_64.zip
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

## Jetson Nano/Ubuntu 18.04

[Optional] You can install an Ubuntu 22.04 VM (5 cores, 8 GB RAM, 32 GB disk) very easily with [multipass](https://multipass.run/):

```bash
multipass launch -n edgeless-jetson -c 5 -m 8G -d 32G 18.04
multipass shell edgeless-jetson
```

Install Rust (follow the interactive instructions):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Install the dependencies and target architectures:

```bash
source "$HOME/.cargo/env"
sudo apt update && sudo apt install gcc libssl-dev pkg-config unzip gcc-aarch64-linux-gnu make g++ -y
rustup target add wasm32-unknown-unknown
rustup target add aarch64-unknown-linux-gnu
cargo install wasm-opt
```

Install the protobuf binaries:

```bash
wget https://github.com/protocolbuffers/protobuf/releases/download/v25.1/protoc-25.1-linux-x86_64.zip
cd /usr/local
sudo unzip $OLDPWD/protoc-25.1-linux-x86_64.zip && sudo rm -f readme.txt
cd -
rm -f protoc-25.1-linux-x86_64.zip
```

At this point you may have to logout/login to let your shell know of the new executable, just try `protoc` to see if it works.

Finally, you can just clone the repo and build the system:

```bash
git clone https://github.com/edgeless-project/edgeless.git
cd edgeless
cargo build --target aarch64-unknown-linux-gnu
```

## Mac OS

Tested on Apple M3 Pro with Sonoma 14.2.1

```bash
brew install protobuf
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup target add wasm32-unknown-unknown
cargo install wasm-opt
cargo build
```
