FROM debian:latest
ARG BIN

WORKDIR /usr/src/myapp

EXPOSE 7011
EXPOSE 7121
EXPOSE 7102

ADD ${BIN}/edgeless_orc_d .
ADD cfg/orchestrator.toml .
RUN echo orchestrator.toml

CMD ["./edgeless_orc_d"]