# Stage 1: Builder
FROM rust:1 AS builder

WORKDIR /app

# Copy Cargo.toml and Cargo.lock first to leverage Docker's build cache.
COPY Cargo.toml Cargo.lock ./

# Create a dummy src/main.rs. This allows Cargo to download and compile
# all dependencies based on Cargo.toml and Cargo.lock before the actual
# source code is copied. This maximizes caching for the 'cargo build' step.
RUN mkdir src && echo "fn main() {println!(\"hello\");}" > src/main.rs

# Build dependencies. This command will be cached unless Cargo.toml or Cargo.lock changes.
# We specify the 'x86_64-unknown-linux-musl' target for static linking,
# which allows us to use a minimal base image in the final stage.
RUN cargo build --release --target x86_64-unknown-linux-musl

# Remove the dummy src directory and its contents to prepare for copying the actual source.
RUN rm -rf src

# Copy the actual Rust source code into the container.
COPY src src/

# Re-run cargo build to compile the application's code.
# Since dependencies are already cached, this step will be faster.
RUN cargo build --release --target x86_64-unknown-linux-musl

# Stage 2: Runner
FROM alpine:latest AS runner

WORKDIR /app

# Copy the compiled release binary from the builder stage.
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/qr-comms ./qr-comms

# Set environment variables for the application.
# RUST_LOG controls the logging level for the `tracing` library.
ENV RUST_LOG=info

ENTRYPOINT ["./qr-comms"]
