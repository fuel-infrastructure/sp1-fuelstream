FROM rust:1.81.0 AS builder

# --------------------------------------------------------
# Builder
# --------------------------------------------------------

WORKDIR /app
COPY .. /app

# Build project
RUN cargo build --bin operator --release

# --------------------------------------------------------
# Runner
# --------------------------------------------------------

FROM rust:1.81.0 AS runner

ENV RUST_LOG=info
ENV TIME_TO_SLEEP_IN_MINUTES=30

WORKDIR /app
# Copy only the necessary files
COPY --from=builder /app/target ./target
COPY --from=builder /app/elf ./elf

ENTRYPOINT ["cargo", "run", "--bin", "operator", "--release"]