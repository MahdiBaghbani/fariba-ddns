# Stage 1: Build stage
FROM rustlang/rust:nightly-bookworm@sha256:d1546a17a1ae256b5d2a82e2296ac6333a979267e69948ab2c1acea9109e883a AS builder

# Install dependencies
RUN apt-get -qq update && \
    apt-get -qq install -y --no-install-recommends curl libssl-dev pkg-config xz-utils && \
    curl -s -L https://github.com/upx/upx/releases/download/v4.2.4/upx-4.2.4-amd64_linux.tar.xz -o upx.tar.xz && \
    tar -xJf upx.tar.xz && \
    mv upx-*/upx /usr/local/bin/ && \
    rm -rf upx.tar.xz upx-*

# Set the working directory
WORKDIR /usr/src/app

# Copy the Cargo.toml and Cargo.lock to the container
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to satisfy cargo fetch and build
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# Pre-fetch dependencies to improve build caching
RUN cargo fetch

# Copy the source code
COPY . .

# Build the application in release mode
RUN cargo build --release

# Compress the resulting binary using UPX
RUN upx --best --ultra-brute --lzma --overlay=strip --force-overwrite target/release/fariba-ddns

# Stage 2: Final minimal image
FROM gcr.io/distroless/cc-debian12@sha256:2fb69596e692931f909c4c69ab09e50608959eaf8898c44fa64db741a23588b0

# Set the working directory
WORKDIR /usr/local/bin

# Copy the compressed binary from the build stage
COPY --from=builder /usr/src/app/target/release/fariba-ddns .

# Set the binary as the default command
CMD ["./fariba-ddns"]
