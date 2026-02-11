# Build stage
FROM rust:1.83-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml ./
COPY server ./server
COPY cli ./cli

# Build release binaries
RUN cargo build --release -p agent-reach-server

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/agent-reach-server /usr/local/bin/

# Create non-root user
RUN useradd -r -s /bin/false agentreach
USER agentreach

EXPOSE 3001

ENV RUST_LOG=agent_reach_server=info
ENV REDIS_URL=redis://localhost:6379

ENTRYPOINT ["agent-reach-server"]
CMD ["--port", "3001"]
