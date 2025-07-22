FROM rust

COPY ./ ./

RUN cargo build --release

CMD ["./target/release/backend_rs"]
