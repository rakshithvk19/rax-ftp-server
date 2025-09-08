# RAX FTP Server

A high-performance, async Rust-based FTP server implementing core features of [RFC 959](https://tools.ietf.org/html/rfc959). Built with Tokio for concurrent client handling and modern systems programming practices.

## Features

- **Async Architecture** - Built on Tokio runtime for high-performance concurrent client handling
- **Complete FTP Protocol** - Full RFC 959 implementation with standard FTP commands
- **Authentication System** - Built-in user management with configurable credentials
- **Dual Connection Modes** - Both active (PORT) and passive (PASV) data connections
- **File Operations** - Upload (STOR), download (RETR), delete (DEL) with progress tracking
- **Directory Management** - List contents (LIST), navigate directories (CWD), print working directory (PWD)
- **Session Management** - USER/PASS authentication, LOGOUT, graceful disconnect (QUIT)
- **Security Features** - IP validation, directory traversal protection, configurable limits
- **Configuration System** - TOML-based config with environment variable overrides
- **Docker Ready** - Complete containerization with multi-stage builds
- **Comprehensive Logging** - Detailed connection, command, and transfer logging
- **Resource Management** - Configurable client limits, file size limits, connection timeouts

## Installation

### Prerequisites
- Rust (stable, edition 2021 or later)
- Tokio runtime

### Build from Source
```bash
git clone <repository-url>
cd rax-ftp-server
cargo build --release
```

### Docker
```bash
docker compose up
```

## Usage

### Local Development
```bash
# Start the server
cargo run --release

# Server starts on 127.0.0.1:2121
# Check logs for connection status
```

### Docker Deployment
```bash
# Using docker-compose (recommended)
docker compose up

# Or build and run manually
docker build -t rax-ftp-server .
docker run -p 2121:2121 -p 2122-2222:2122-2222 rax-ftp-server
```

### Testing with FTP Client
```bash
# Connect with any FTP client
ftp 127.0.0.1 2121

# Or test with telnet/netcat
telnet 127.0.0.1 2121
```

Example session:
```
220 Welcome to RAX FTP Server
USER alice
331 Password required
PASS alice123
230 Login successful
PASV
227 Entering Passive Mode (127,0,0,1,8,79)
STOR myfile.txt
150 Opening BINARY mode data connection for file transfer
226 Transfer complete
LIST
150 Opening ASCII mode data connection for file list
226 Directory send OK
QUIT
221 Goodbye
```

## Supported Commands

| Command | Description | Example |
|---------|-------------|---------|
| `USER <username>` | Specify username for authentication | `USER alice` |
| `PASS <password>` | Specify password for authentication | `PASS alice123` |
| `STOR <filename>` | Upload file to server | `STOR document.pdf` |
| `RETR <filename>` | Download file from server | `RETR report.txt` |
| `LIST` | List directory contents | `LIST` |
| `DEL <filename>` | Delete file on server | `DEL oldfile.txt` |
| `PWD` | Print working directory | `PWD` |
| `CWD <directory>` | Change working directory | `CWD /subfolder` |
| `PORT <ip:port>` | Set active mode data connection | `PORT 127.0.0.1:8080` |
| `PASV` | Enter passive mode | `PASV` |
| `LOGOUT` | Log out current user (keeps connection) | `LOGOUT` |
| `RAX` | Custom server command | `RAX` |
| `QUIT` | Disconnect from server | `QUIT` |

## Authentication

The server includes built-in user accounts:

| Username | Password | Description |
|----------|----------|-------------|
| `alice` | `alice123` | Standard user account |
| `bob` | `bob123` | Standard user account |
| `admin` | `admin123` | Administrator account |

## Configuration

### Configuration File (config.toml)
```toml
# Network configuration
bind_address = "127.0.0.1"
control_port = 2121
data_port_min = 2122
data_port_max = 2222

# Client and resource limits
max_clients = 10
max_file_size_mb = 100

# Server settings
server_root = "./server_root"
buffer_size = 8192
connection_timeout_secs = 10

# Security settings
max_directory_depth = 3
max_username_length = 64
min_client_port = 1024
```

### Environment Variables
Override any config value with `RAX_FTP_` prefixed environment variables:

```bash
# Network settings
export RAX_FTP_BIND_ADDRESS=0.0.0.0
export RAX_FTP_CONTROL_PORT=21
export RAX_FTP_SERVER_ROOT=/app/ftp_root

# Resource limits  
export RAX_FTP_MAX_CLIENTS=20
export RAX_FTP_MAX_FILE_SIZE_MB=500
export RAX_FTP_DATA_PORT_MIN=2122
export RAX_FTP_DATA_PORT_MAX=2222
```

## Connection Modes

### Passive Mode (PASV) - Recommended
Server creates a data connection listener and tells the client where to connect:
```
PASV
227 Entering Passive Mode (127,0,0,1,8,79)
```

### Active Mode (PORT)
Client tells server where to connect for data transfers:
```
PORT 127.0.0.1:8080
200 PORT command successful
```

## Docker Configuration

### docker-compose.yml
```yaml
services:
  rax-ftp-server:
    build: .
    ports:
      - "2121:2121"           # Control port
      - "2122-2222:2122-2222" # Data port range
    environment:
      - RUST_LOG=info
      - RAX_FTP_BIND_ADDRESS=0.0.0.0
      - RAX_FTP_SERVER_ROOT=/app/rax-ftp-server/server_root
    volumes:
      - ./server_root:/app/rax-ftp-server/server_root
```

### Key Docker Settings
- Use `RAX_FTP_BIND_ADDRESS=0.0.0.0` for containers
- Mount server root directory as volume for persistence
- Map both control port (2121) and data port range (2122-2222)

## Security Features

- **IP Validation** - PORT command validates client IP matches connection IP
- **Directory Traversal Protection** - Prevents access outside server root
- **Configurable Limits** - File size, client count, directory depth limits
- **Port Range Validation** - Enforces minimum port numbers for security
- **Username Length Limits** - Prevents buffer overflow attacks
- **Connection Timeouts** - Automatic cleanup of stale connections

## Logging

Enable detailed logging with the `RUST_LOG` environment variable:

```bash
# Different log levels
export RUST_LOG=info    # Standard logging
export RUST_LOG=debug   # Detailed debugging
export RUST_LOG=warn    # Warnings and errors only

# Module-specific logging
export RUST_LOG=rax_ftp_server::client=debug,rax_ftp_server::transfer=info
```

Log categories:
- **Connection logs** - Client connect/disconnect events
- **Authentication logs** - Login/logout attempts
- **Command logs** - All FTP commands received
- **Transfer logs** - File upload/download operations
- **Error logs** - Error conditions and recovery

## Architecture

```
src/
├── main.rs                 # Server entry point
├── config.rs              # Configuration management
├── server/                 # Core server implementation
├── client/                 # Client state and session management
├── protocol/               # FTP command parsing and handling
├── auth/                   # Authentication system
├── transfer/               # Data channel and file transfer operations
├── storage/                # File system operations
├── navigate/               # Directory navigation
└── error/                  # Error handling
```

## Performance

- **Async I/O** - Non-blocking operations for high concurrency
- **Connection Pooling** - Efficient client connection management  
- **Buffered Transfers** - Configurable buffer sizes for optimal throughput
- **Resource Limits** - Prevents resource exhaustion under load
- **Persistent Data Channels** - Reusable data connections for multiple operations

## Error Handling

The server provides detailed error responses following FTP standards:

- **4xx Errors** - Temporary failures, client should retry
- **5xx Errors** - Permanent failures, client should not retry
- **Connection Management** - Graceful handling of connection drops
- **Resource Cleanup** - Automatic cleanup of failed operations

## File Structure
```
server_root/                # FTP root directory
├── uploads/               # Client uploaded files
├── downloads/             # Files available for download
└── shared/                # Shared directory
```

## License

MIT License. See [LICENSE](LICENSE) for details.

## Related Projects

- [rax-ftp-client](https://github.com/rakshithvk19/rax-ftp-client) - The companion FTP client

---

**Note**: This server is designed for learning and development purposes. For production use, consider additional security measures such as TLS/SSL encryption, database-backed authentication, and comprehensive access controls.