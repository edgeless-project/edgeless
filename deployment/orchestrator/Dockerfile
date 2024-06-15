FROM rust:1.77

WORKDIR /usr/src/myapp

RUN apt-get update -y
RUN apt-get install protobuf-compiler -y

RUN git clone https://github.com/edgeless-project/edgeless.git
WORKDIR /usr/src/myapp/edgeless/
WORKDIR /usr/src/myapp/edgeless/edgeless_orc
RUN git fetch -p
RUN git branch -lr
RUN git checkout 149-ε-orc-delegated-orchestration-model
RUN cargo build --release

ADD start.sh /
RUN chmod +x /start.sh

CMD ["/start.sh"]