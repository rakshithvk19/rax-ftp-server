# Stage 1: Build
FROM rust:1.88 AS builder  
WORKDIR /rax-ftp-server
COPY . .
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Create project directory structure
RUN mkdir -p /app/rax-ftp-server/server_root

# Copy binary and config from builder stage
COPY --from=builder /rax-ftp-server/target/release/rax-ftp-server /app/rax-ftp-server/
COPY --from=builder /rax-ftp-server/config.toml /app/rax-ftp-server/

# FTP server ports
EXPOSE 2121 
EXPOSE 2122-2222

# Volume mount for persistent FTP files
VOLUME ["/app/rax-ftp-server/server_root"]

# Container environment overrides
ENV RAX_FTP_BIND_ADDRESS=0.0.0.0
ENV RAX_FTP_SERVER_ROOT=/app/rax-ftp-server/server_root

# Run FTP server
CMD ["./rax-ftp-server/rax-ftp-server"]