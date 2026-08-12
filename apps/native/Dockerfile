FROM rust:1.97-slim-bookworm AS builder

WORKDIR /source
COPY . .
RUN cargo build --locked --release -p forum-native

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /source/target/release/forum-native /usr/local/bin/forum-native

USER 10001:10001
ENV LISTEN_ADDR=0.0.0.0:3000
EXPOSE 3000
ENTRYPOINT ["forum-native"]
