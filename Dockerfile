FROM rust:1-slim-buster AS builder

WORKDIR /usr/src/rinha

COPY ./app/Cargo.toml ./app/Cargo.lock .

RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo fetch
RUN cargo build --release
RUN rm src/main.rs

COPY ./app/ .
ENV SQLX_OFFLINE true
RUN touch src/main.rs
RUN cargo install --path .

FROM debian:bullseye-slim
COPY --from=builder /usr/local/cargo/bin/rinha /usr/local/bin/rinha
CMD ["rinha"]
