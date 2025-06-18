# Stage 1: Build the Rust project for x86_64
FROM rust:bookworm AS builder

RUN --mount=type=cache,sharing=private,target=/var/cache/apt <<EOT
#!/bin/bash
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

apt-get update
apt-get install -y --no-install-recommends git
EOT
WORKDIR /app

FROM builder AS build

RUN --mount=type=bind,target=/app \
    --mount=type=cache,sharing=private,target=/app/target \
<<EOT
#!/bin/bash
set -euo pipefail

rm -rf /out
mkdir /out

cargo build --release --package tfinit

cp /app/target/release/tfinit /out/

EOT

FROM debian:bookworm

WORKDIR /app

# Copy the binary from the builder stage
COPY --from=build /out/tfinit /usr/local/bin/tfinit

# Set the entrypoint to the binary
ENTRYPOINT ["tfinit"]

# Show help message when container is run without arguments
CMD ["--help"]
