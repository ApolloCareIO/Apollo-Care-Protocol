# Apollo Care Protocol - Development Dockerfile
# Build: docker build -t apollo-care-dev .
# Run: docker run -it --rm -v $(pwd):/workspace apollo-care-dev

FROM --platform=linux/amd64 ubuntu:22.04

# Prevent interactive prompts
ENV DEBIAN_FRONTEND=noninteractive

# Install system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    libudev-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    RUST_VERSION=1.75.0

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain $RUST_VERSION \
    && rustup component add rustfmt clippy

# Install Node.js
ENV NODE_VERSION=20
RUN curl -fsSL https://deb.nodesource.com/setup_${NODE_VERSION}.x | bash - \
    && apt-get install -y nodejs \
    && npm install -g yarn

# Install Solana CLI
ENV SOLANA_VERSION=1.18.17
RUN sh -c "$(curl -sSfL https://release.solana.com/v${SOLANA_VERSION}/install)" \
    && /root/.local/share/solana/install/active_release/bin/solana --version

ENV PATH="/root/.local/share/solana/install/active_release/bin:${PATH}"

# Install Anchor CLI
ENV ANCHOR_VERSION=0.30.1
RUN cargo install --git https://github.com/coral-xyz/anchor avm --locked --force \
    && avm install ${ANCHOR_VERSION} \
    && avm use ${ANCHOR_VERSION}

# Create workspace
WORKDIR /workspace

# Generate a default keypair for development
RUN solana-keygen new --no-bip39-passphrase -o /root/.config/solana/id.json

# Set default cluster to localnet
RUN solana config set --url localhost

# Expose ports for local validator
EXPOSE 8899 8900

# Default command
CMD ["bash"]
