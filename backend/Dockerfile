# Stage 1: Builder
FROM rust:1 AS builder

WORKDIR /app

# Copy Cargo.toml and Cargo.lock
COPY Cargo.toml Cargo.lock ./
# Copy the actual Rust source code into the container.
COPY src src/

# Build dependencies. This command will be cached unless Cargo.toml or Cargo.lock changes.
# We specify the 'x86_64-unknown-linux-musl' target for static linking,
# which allows us to use a minimal base image in the final stage.
RUN rustup target add x86_64-unknown-linux-musl

# Run cargo build to compile the application's code.
RUN cargo build --release --target x86_64-unknown-linux-musl

# Stage 2: Runner
FROM alpine:latest AS runner

WORKDIR /app

# Copy the compiled release binary from the builder stage.
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/qr_comms_backend ./qr_comms_backend

# Create a non-root user and group
RUN addgroup -S appgroup && adduser -S appuser -G appgroup

# Change ownership of the application to the new user
RUN chown appuser:appgroup ./qr_comms_backend

# Switch to the non-root user
USER appuser

# Set environment variables for the application.
# RUST_LOG controls the logging level for the `tracing` library.
ENV RUST_LOG=info

ENTRYPOINT ["./qr_comms_backend"]
