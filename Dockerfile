
FROM rustlang/rust:nightly-bullseye AS builder

ARG SQLX_OFFLINE=true
ENV SQLX_OFFLINE=${SQLX_OFFLINE}

WORKDIR /usr/src/mini_dex_core
RUN cargo init

COPY Cargo.toml Cargo.lock ./
RUN cargo build --release


COPY .sqlx ./.sqlx

COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/mini_dex_core/target/release/mini_dex_core .

EXPOSE 3000

CMD ["./mini_dex_core"]