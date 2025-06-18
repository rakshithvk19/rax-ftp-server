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

- `src/main.rs`: Initializes logging and starts the server.
- `src/server.rs`: Manages control connections (`TcpListener` on 2121) and client threads.
- `src/client.rs`: Handles client state (authentication, data connections).
- `src/commands/`:
  - `parser.rs`: Parses FTP commands.
  - `handlers.rs`: Executes commands.
- `tests/`: Unit and integration tests.

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

## Testing

- Run automated tests:
  ```bash
  cargo test
  ```
  - Covers server startup and basic command parsing.

- Manual testing:
  - Use `telnet`/`netcat` as above for `PORT`, `PASV`, `STOR`, `RETR`, `LIST`, etc.
  - Test invalid inputs (e.g., `PORT 192.168.1.100::2122` → `501 Invalid IP address`).
  - Verify concurrent clients by opening multiple `telnet` sessions.

## Known Limitations

- **Hardcoded Credentials**: Single user (`user/pass`) for authentication.
- **Data Channel Closure**: `PORT` and `PASV` data connections close after each transfer.
- **Basic Error Handling**: Limited validation for `CWD` paths.
- **IPv4 Only**: `PORT` supports IPv4 addresses.

## Future Work

- **Week 6**: Enhance `CWD` validation, add timeout for `PORT` connections.
- **Week 7–8**: Implement persistent data channels for `PORT`/`PASV`.
- **Week 9–10**: Transition to Tokio for async I/O, add admin commands (e.g., list clients).
- **Testing**: Add integration tests for all commands and concurrency.

## Contributing

This is a personal portfolio project. Feedback is welcome via GitHub issues or pull requests.

## License

MIT License. See [LICENSE](LICENSE) for details.

