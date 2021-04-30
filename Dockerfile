FROM rust:latest as builder
WORKDIR build
ADD . .
RUN cargo build --release

FROM gcr.io/distroless/cc
WORKDIR app
COPY --from=builder /build/target/release/archbot-rs /app/
CMD ["/app/archbot-rs"]
