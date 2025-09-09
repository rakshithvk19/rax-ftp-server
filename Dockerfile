# Stage 1: Build
FROM rust:1.88 AS builder  
WORKDIR /rax-ftp-server
COPY . .
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Install utilities for generating sample files
RUN apt-get update && apt-get install -y \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create project directory structure
RUN mkdir -p /app/rax-ftp-server/server_root

# Copy binary and config from builder stage
COPY --from=builder /rax-ftp-server/target/release/rax-ftp-server /app/rax-ftp-server/
COPY --from=builder /rax-ftp-server/config.toml /app/rax-ftp-server/

# Create default directories in server_root
RUN mkdir -p /app/rax-ftp-server/server_root/uploads \
    /app/rax-ftp-server/server_root/downloads \
    /app/rax-ftp-server/server_root/samples

# Create sample text files
RUN echo "Welcome to RAX FTP Server!" > /app/rax-ftp-server/server_root/samples/readme.txt && \
    echo "This is a test file for FTP operations." > /app/rax-ftp-server/server_root/samples/test.txt && \
    echo "Lorem ipsum dolor sit amet, consectetur adipiscing elit." > /app/rax-ftp-server/server_root/samples/document.txt

# Create a sample CSV file
RUN printf "Name,Age,City\nAlice,30,New York\nBob,25,Los Angeles\nCharlie,35,Chicago\n" > /app/rax-ftp-server/server_root/samples/data.csv

# Create a sample JSON file
RUN printf '{\n  "server": "RAX-FTP",\n  "version": "1.0",\n  "features": ["LIST", "STOR", "RETR", "PASV", "PORT"]\n}' > /app/rax-ftp-server/server_root/samples/config.json

# Download a small sample PDF (Lorem Ipsum PDF)
RUN curl -L "https://www.w3.org/WAI/ER/tests/xhtml/testfiles/resources/pdf/dummy.pdf" \
    -o /app/rax-ftp-server/server_root/samples/sample.pdf 2>/dev/null || \
    echo "%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources 4 0 R /MediaBox [0 0 612 792] /Contents 5 0 R >>\nendobj\n4 0 obj\n<< /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >>\nendobj\n5 0 obj\n<< /Length 44 >>\nstream\nBT /F1 12 Tf 100 700 Td (Sample PDF) Tj ET\nendstream\nendobj\nxref\n0 6\ntrailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n0\n%%EOF" > /app/rax-ftp-server/server_root/samples/sample.pdf

# Create a small sample MP3 (silent audio file - very small)
RUN printf "\xFF\xFB\x90\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00" > /app/rax-ftp-server/server_root/samples/audio.mp3

# Create welcome message in downloads directory
RUN echo "Downloaded files will appear here" > /app/rax-ftp-server/server_root/downloads/info.txt

# Create upload instructions
RUN echo "Upload your files to this directory" > /app/rax-ftp-server/server_root/uploads/info.txt

# FTP server ports
EXPOSE 2121 
EXPOSE 2122-2222

# Volume mount for persistent FTP files
VOLUME ["/app/rax-ftp-server/server_root"]

# Container environment overrides
ENV RAX_FTP_SERVER_ROOT=/app/rax-ftp-server/server_root

# Run FTP server
CMD ["./rax-ftp-server/rax-ftp-server"]