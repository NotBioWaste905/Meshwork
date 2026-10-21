FROM rust:1.99

COPY ./ ./

RUN cargo build --release

CMD ["./target/release/backend_rs"]
