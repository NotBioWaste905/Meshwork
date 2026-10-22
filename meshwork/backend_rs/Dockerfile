# Build stage: pinned toolchain. The runtime stage must use the same Debian
# release as the build image (trixie), otherwise the binary may need a newer glibc.
FROM rust:1.99-trixie AS build

WORKDIR /app

# Cache dependencies: build them against stub sources first, so editing
# src/ does not recompile every crate.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && echo 'fn main() {}' > src/main.rs \
    && touch src/lib.rs \
    && cargo build --release \
    && rm -rf src

COPY src ./src
# Make sure cargo sees the real sources as newer than the stub build.
RUN touch src/main.rs src/lib.rs && cargo build --release


FROM debian:trixie-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --system --no-create-home meshwork
COPY --from=build /app/target/release/backend_rs /usr/local/bin/backend_rs
# Issues actor API tokens: docker compose exec backend meshwork-token create <actor>
COPY --from=build /app/target/release/meshwork-token /usr/local/bin/meshwork-token

USER meshwork
EXPOSE 8000
CMD ["backend_rs"]
