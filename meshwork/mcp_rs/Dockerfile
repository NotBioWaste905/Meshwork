FROM rust:1.99 AS build
WORKDIR /src
COPY ./ ./
RUN cargo build --release

# Same Debian release as rust:1.99, so glibc matches
FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/mcp_rs /usr/local/bin/mcp_rs
CMD ["mcp_rs"]
