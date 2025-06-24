# RAX FTP Server

A Rust-based File Transfer Protocol (FTP) server implementing core features of RFC 959. This project showcases proficiency in Rust, TCP networking, multi-threading, and modular design, built as a portfolio piece to demonstrate systems programming skills.

## Features

- **Control Connection**: Listens on `127.0.0.1:2121` for FTP commands via TCP.
- **Authentication**: Supports `USER`, `PASS`, and `LOGOUT` with hardcoded credentials (`user/pass`).
- **File Operations**:
  - `STOR`: Upload files from client to server.
  - `RETR`: Download files from server to client.
  - `LIST`: Retrieve directory listings.
- **Data Connections**:
  - `PORT`: Active mode with `ip::port` format (e.g., `127.0.0.1::2122`), includes IP validation for security.
  - `PASV`: Passive mode, server listens on dynamic ports (2122–2222).
- **Directory Navigation**:
  - `CWD`: Change working directory.
  - `PWD`: Print working directory.
- **Session Management**: `QUIT` for graceful disconnection.
- **Multi-Threading**: Handles multiple clients concurrently using `std::thread` and `Arc<Mutex<HashMap>>` for thread-safe client state.
- **Client Tracking**: Manages client connections with IP-based identification.
- **Logging**: Detailed connection and command logs via the `log` crate.

## Project Structure

- `main.rs`: Initializes logging and starts the FTP server.
- `server.rs`: Implements the main FTP server, managing client connections, enforcing a client limit, and spawning handler threads for each new client.
- `client.rs`: Defines the Client struct and its methods for managing FTP client state and authentication.
- `client_handler.rs`: Handles client connections and processes FTP commands.
- `command.rs`: Defines command parsing logic and related data structures for handling FTP commands.
- `handlers.rs`: Defines handlers for FTP commands, coordinating authentication, file operations, directory management, and data channel setup for each client.
- `auth.rs`: Provides authentication logic and error handling for FTP users.
- `channel_registry.rs`: Manages a registry of data channels for FTP connections.
- `data_channel.rs`: Manages accepting and handling data connections for file transfers in an FTP-like server, coordinating client-specific TCP listeners.
- `file_transfer.rs`: Handles file upload and download operations for the FTP server.
- `lib.rs`: Library root, re-exports modules and the server.

## Requirements

- Rust (stable, edition 2021 or later)
- Cargo
- `telnet` and `netcat` (`nc`) for testing
- Dependencies (in `Cargo.toml`):
  ```toml
  [dependencies]
  log = "0.4"
  env_logger = "0.9"
  ```

## Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/your-username/rax-ftp-server
   cd rax-ftp-server
   ```

2. Install Rust and Cargo:
   - Follow [rustup.rs](https://rustup.rs/) instructions.

3. Install testing tools:
   - Linux: `sudo apt install telnet netcat`
   - macOS: `brew install telnet netcat`

4. Build the project:
   ```bash
   cargo build --release
   ```

## Usage

1. Start the server:
   ```bash
   cargo run --release
   ```
   - Listens on `127.0.0.1:2121`.
   - Logs show connections and commands.

2. Test with `telnet` and `netcat`:
   - **Active Mode (`PORT`) Example**:
     - **Data Connection** (terminal 1):
       ```bash
       echo "Hello, FTP!" > test.txt
       nc -l 2122 < test.txt
       ```
     - **Control Connection** (terminal 2):
       ```bash
       telnet 127.0.0.1 2121
       ```
       ```
       USER user
       PASS pass
       PORT 127.0.0.1::2122
       STOR uploaded.txt
       QUIT
       ```
       - Expected:
         - `220 Welcome to RAX FTP server`
         - `230 Login successful`
         - `200 PORT command successful`
         - `150 Opening data connection`, `226 Transfer complete`
         - `221 Goodbye`
       - Verify: `cat uploaded.txt` shows `Hello, FTP!`.

   - **Passive Mode (`PASV`) Example**:
     - **Control Connection** (terminal 1):
       ```bash
       telnet 127.0.0.1 2121
       ```
       ```
       USER user
       PASS pass
       PASV
       ```
       - Note the response, e.g., `227 Entering Passive Mode (127.0.0.1:2122)`.
       ```
       STOR uploaded.txt
       QUIT
       ```
     - **Data Connection** (terminal 2, after `PASV`):
       ```bash
       echo "Hello, FTP!" > test.txt
       nc 127.0.0.1 2122 < test.txt
       ```
       - Verify: `cat uploaded.txt`.

   - **Other Commands**:
     - `RETR test.txt`: Download `test.txt` (use `nc -l 2122 > retrieved.txt` for `PORT`).
     - `LIST`: View directory (use `nc -l 2122 > listing.txt` for `PORT`).
     - `CWD dir`: Change to `dir`.
     - `PWD`: Show current directory.
     - `LOGOUT`: Reset session.

3. Check logs for client IPs, commands, and transfer status.

## Supported Commands

| Command | Description 
|---------|-------------
| `USER` | Specify username 
| `PASS` | Specify password 
| `QUIT` | Disconnect from server 
| `LOGOUT` | Log out current user 
| `LIST` | Directory listing 
| `PWD` | Print working directory 
| `CWD` | Change working directory 
| `RETR` | Download/retrieve file 
| `STOR` | Upload/store file 
| `DEL` | Delete file 
| `PORT` | Specifies the client-side IP address and port number for the server to connect to for the upcoming data transfer. This enables active mode, where the client listens for a data connection and the server initiates the connection to the provided address and port. Format: PORT <ip>::<port> (e.g., PORT 127.0.0.1::2122).
| `PASV` | Instructs the server to enter passive mode by opening a new listening port for data transfer. The server responds with the IP address and port number, and the client then initiates the data connection to this address. This is useful when the client is behind a firewall or NAT and cannot accept incoming connections.
|

## License

MIT License. See [LICENSE](LICENSE) for details.

