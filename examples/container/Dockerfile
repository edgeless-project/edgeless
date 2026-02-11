FROM rust:1.90

WORKDIR /usr/src/myapp

RUN apt-get update -y
RUN apt-get install protobuf-compiler curl git gcc libssl-dev pkg-config unzip make g++ libtss2-dev -y

RUN git clone https://github.com/edgeless-project/edgeless.git
WORKDIR /usr/src/myapp/edgeless/
WORKDIR /usr/src/myapp/edgeless/edgeless_container_function
RUN cargo build --release

EXPOSE 7101

ENV RUST_LOG=info

CMD ["/usr/src/myapp/edgeless/target/release/edgeless_container_function_d", "--endpoint", "http://0.0.0.0:7101/"]
