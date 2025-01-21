FROM rust:1.81.0 AS builder

# --------------------------------------------------------
# Builder
# --------------------------------------------------------

WORKDIR /app
COPY . /app

# Build project
RUN cargo build --bin operator --release

# --------------------------------------------------------
# Runner
# --------------------------------------------------------

FROM alpine AS runner

ENV RUST_LOG=info
ENV TIME_TO_SLEEP_IN_MINUTES=30

WORKDIR /app

# Copy only the necessary files
COPY --from=builder /app/target/release/operator ./operator
COPY --from=builder /app/elf ./elf

# Install bash for wait-for-it script
RUN apk add --no-cache bash

# Give permissions to wait-for-it.sh
COPY scripts/wait-for-it.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/wait-for-it.sh

# Run the command and wait to delay the restart
ENTRYPOINT [ "sh", "-c", "wait-for-it.sh -t ${TIME_TO_SLEEP_IN_MINUTES} -- RUST_LOG=${RUST_LOG} ./operator" ]