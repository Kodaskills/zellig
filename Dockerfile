# ── Stage 1: cargo-chef base ─────────────────────────────────────────────────
FROM rust:1.88-slim-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

# ── Stage 2: dependency planner ──────────────────────────────────────────────
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ── Stage 3: release builder (with local feature) ────────────────────────────
# CTranslate2 and DNNL are both compiled from vendored source and linked
# statically — no system native library installation needed at runtime.
FROM chef AS builder
RUN apt-get update && apt-get install -y --no-install-recommends \
    cmake build-essential pkg-config \
    && rm -rf /var/lib/apt/lists/*
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --features local --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin zellig --features local

# ── Stage 4: dev — local feature + hot-reload ────────────────────────────────
FROM chef AS dev
RUN apt-get update && apt-get install -y --no-install-recommends \
    cmake build-essential pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-watch --locked
COPY . .
RUN cargo build --features local
ENV RUST_LOG=debug
CMD ["cargo", "watch", "-x", "check --features local"]

# ── Stage 5: prod — self-contained binary, no native deps ────────────────────
FROM debian:bookworm-slim AS prod
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libgomp1 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/zellig /usr/local/bin/zellig
ENTRYPOINT ["zellig"]
CMD ["--help"]
