FROM rust:1.85

WORKDIR /usr/src/myapp

RUN apt-get update -y
RUN apt-get install protobuf-compiler uuid -y

RUN git clone https://github.com/edgeless-project/edgeless.git
WORKDIR /usr/src/myapp/edgeless/
WORKDIR /usr/src/myapp/edgeless/edgeless_node
RUN git fetch -p
RUN git branch -lr
RUN cargo build --release

ADD start.sh /
RUN chmod +x /start.sh

CMD ["/start.sh"]