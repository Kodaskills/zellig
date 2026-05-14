# ── Stage 1: dependency planner (standard Rust, fast recipe gen) ─────────────
FROM rust:1.88-slim-bookworm AS planner
RUN cargo install cargo-chef --locked
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ── Stage 2: release builder (CUDA devel — one binary: DNNL + CUDA dynamic) ──
# Requires no real GPU; nvcc + CUDA headers are enough at build time.
# At runtime the binary auto-detects CUDA and falls back to CPU if absent.
FROM nvidia/cuda:12.6.3-devel-ubuntu22.04 AS builder
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl build-essential cmake pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:$PATH"
RUN cargo install cargo-chef --locked
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --features cuda --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin zellig --features cuda

# ── Stage 3: dev — local feature + hot-reload ────────────────────────────────
FROM rust:1.88-slim-bookworm AS dev
RUN apt-get update && apt-get install -y --no-install-recommends \
    cmake build-essential pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-watch --locked
WORKDIR /app
COPY . .
RUN cargo build --features local
ENV RUST_LOG=debug
CMD ["cargo", "watch", "-x", "check --features local"]

# ── Stage 4: prod — CUDA runtime so GPU libs are present when GPU is available ─
FROM nvidia/cuda:12.6.3-runtime-ubuntu22.04 AS prod
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libgomp1 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/zellig /usr/local/bin/zellig
ENTRYPOINT ["zellig"]
CMD ["--help"]
