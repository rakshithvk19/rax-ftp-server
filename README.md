# RAX FTP Server

A Rust-based File Transfer Protocol (FTP) server implementing core features of RFC 959. This project demonstrates proficiency in Rust, TCP networking, multi-threading, and modular design, built as a portfolio piece to showcase systems programming skills.

## Features

**Control Connection**: Listens on `127.0.0.1:2121` for FTP commands using TCP.

- **Authentication**: Supports `USER` and `PASS` commands with basic user validation.
- **File Upload**: Implements `STOR` to upload files from client to server via a data connection.
- **Active Mode**: Supports `PORT` command for client-specified data ports (e.g., `127.0.0.1:2122`), enabling concurrent transfers.
- **Multi-Threading**: Handles multiple clients concurrently using `std::thread`, with per-client state isolation.
- **Client Tracking**: Captures IP addresses of connected clients using a thread-safe `HashSet`.
- **Shutdown**: Supports graceful server shutdown with `SHUTDOWN` command (admin-only).
- **Logging**: Uses the `log` crate for detailed connection and command logging.
- **Temporary Hardcoding**: `STOR` currently uses hardcoded ports for testing (to be replaced with dynamic `PORT` handling).

## Project Structure

- `src/main.rs`: Entry point, initializes logging and starts the server.
- `src/server.rs`: Manages control connections (`TcpListener` on 2121), client threads, and client IP tracking.
- `src/connection.rs`: Handles data connections for `STOR` (active mode via `PORT`).
- `src/client.rs`: Manages user authentication state (`Client`).
- `src/commands/`: Parses and handles FTP commands (`USER`, `PASS`, `STOR`, `PORT`, etc.).
  - `mod.rs`: Command parsing logic.
  - `handlers.rs`: Command execution.
  - `utils.rs`: Response utilities.
- `tests/`: Integration tests for server startup and concurrent connections.

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

## Setup

1. Clone the repository:

   ```bash
   git clone <repository-url>
   cd rax-ftp-server
   ```

2. Install Rust and Cargo:

   - Follow rustup.rs instructions.

3. Install testing tools:

   - Linux: `sudo apt install telnet netcat`
   - macOS: `brew install telnet netcat`

4. Build the project:

   ```bash
   cargo build --release
   ```

## Usage

1. Run the server:

   ```bash
   cargo run --release
   ```

   - Server listens on `127.0.0.1:2121`.
   - Logs display connections and commands.

2. Test with `telnet` and `netcat`:

   - **Data Connection** (in a terminal):

     ```bash
     echo "Hello, FTP!" > test.txt
     nc -l 2122 < test.txt
     ```

     - Listens on port 2122, sends `test.txt` when connected.

   - **Control Connection** (in another terminal):

     ```bash
     telnet 127.0.0.1 2121
     ```

     - Commands:

       ```
       USER user
       PASS pass
       PORT 127,0,0,1,8,90  # Specifies 127.0.0.1:2122 (8*256+90=2122)
       STOR uploaded.txt
       QUIT
       ```

     - Expected:

       - `220 Welcome to RAX FTP server !!`
       - `230 Login successful`
       - `200 PORT command successful`
       - `150 Opening data connection`, then `226 Transfer complete`
       - `221 Goodbye`

     - Verify: `uploaded.txt` in server directory contains `Hello, FTP!`.

3. Check logs for client IPs and connection status.

## Testing

- Run unit and integration tests:

  ```bash
  cargo test
  ```

- Tests include:

  - Server startup and welcome message.
  - Concurrent client connections.

- Manual testing with `telnet`/`netcat` verifies `STOR` and `PORT`.

## Known Limitations

- **Hardcoded Ports**: `STOR` data connection setup is partially hardcoded for testing; full `PORT` integration is in progress.
- **Limited Commands**: Only `USER`, `PASS`, `STOR`, `PORT`, `QUIT`, and `SHUTDOWN` are implemented.
- **Single User**: Authentication supports one hardcoded user (`user/pass`).
- **Active Mode Only**: `PASV` (passive mode) is not yet implemented.

## Future Work

- **Week 5**: Complete `PORT` integration for dynamic data ports, replace hardcoded logic.
- **Week 6**: Implement `PASV` for passive mode, add `RETR` (download) and `LIST` (directory listing).
- **Week 7–8**: Add `CWD`, `PWD`, and directory validation for `STOR`.
- **Week 9–10**: Transition to Tokio for async I/O, improve error handling, and add admin commands (e.g., list connected clients).
- **Testing**: Expand integration tests for all commands and concurrency scenarios.

## Contributing

This is a personal project for learning and portfolio purposes. Feedback is welcome via issues or pull requests.

## License

MIT License. See LICENSE for details.
