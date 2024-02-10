FROM rust:1-slim-buster AS builder

WORKDIR /usr/src/rinha

COPY ./app/ .
ENV SQLX_OFFLINE true
RUN cargo install --path .

FROM debian:bullseye-slim
COPY --from=builder /usr/local/cargo/bin/rinha /usr/local/bin/rinha
CMD ["rinha"]
