FROM rust:1.80-bookworm AS rust_builder

ADD . /sources

WORKDIR /sources

RUN cargo build --all-features --release

FROM debian:bookworm

RUN apt-get update && \
   apt-get install -y \
     ca-certificates

COPY --from=rust_builder /sources/target/release/server-and-database /usr/bin

CMD server-and-database
