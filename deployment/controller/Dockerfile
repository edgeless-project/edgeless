FROM rust:1.85

WORKDIR /usr/src/myapp

RUN apt-get update -y
RUN apt-get install protobuf-compiler -y

RUN git clone https://github.com/edgeless-project/edgeless.git
WORKDIR /usr/src/myapp/edgeless/
WORKDIR /usr/src/myapp/edgeless/edgeless_con
RUN git fetch -p
RUN git branch -lr
RUN cargo build --release

EXPOSE 7001

ADD start.sh /
RUN chmod +x /start.sh

CMD ["/start.sh"]