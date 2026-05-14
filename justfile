default:
    @just --list

# Format Rust code
[group('quality')]
fmt:
    cargo fmt --all

# Lint Rust code
[group('quality')]
lint:
    cargo clippy --all-targets -- -D warnings

# Check for outdated dependencies
[group('quality')]
outdated:
    cargo outdated --depth 1

# Check all quality groups
[group('quality')]
check: fmt lint test doc audit outdated

# Run tests
[group('tests')]
test:
    cargo test --all-targets

# Generate documentation
[group('doc')]
doc:
    cargo doc --no-deps

# Run security audit
[group('security')]
audit:
    cargo audit

# ── Docker (unified dev & prod) ───────────────────────────#

# Build and push prod image to Docker Hub. Usage: just push [tag]
[group('docker')]
push tag="latest":
    docker build --target prod -t kodaskills/zellig:{{ tag }} .
    docker push kodaskills/zellig:{{ tag }}

# Run any zellig command in dev|prod. Default: dev
[group('docker')]
[positional-arguments]
up service="dev" *args:
    #!/usr/bin/env sh
    if [ "$#" -le 1 ]; then
        docker compose up "$1" --build
    else
        docker compose run --rm "$@"
    fi

# Build an image without starting it. Default: dev
[group('docker')]
build service="dev":
    docker compose build {{ service }}

# Stop **all** running services (docker compose down)
[group('docker')]
down:
    docker compose down

# View logs of a service. Default: dev
[group('docker')]
logs service="dev":
    docker compose logs -f {{ service }}

# Open a shell inside a running service. Default: dev
[group('docker')]
shell service="dev":
    docker compose exec {{ service }} /bin/bash
