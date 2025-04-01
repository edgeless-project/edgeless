FROM golang:1.24 AS builder

WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN go build -o main .

# NOTE: golang base image uses glibc, alpine requires musl
FROM debian:bookworm-slim
WORKDIR /root/
COPY --from=builder /app/main .

CMD ["./main"]