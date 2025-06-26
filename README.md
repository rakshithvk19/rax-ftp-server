# RAX FTP Server

**A Rust-based FTP server implementing core features of [RFC 959](https://tools.ietf.org/html/rfc959).**

This project demonstrates robust systems programming skills with Rust, including TCP networking, multi-threading, modular design, and practical FTP protocol handling. It is designed as a portfolio piece showcasing backend and network programming expertise.

---

## Table of Contents

- [Features](#features)  
- [Project Structure](#project-structure)  
- [Requirements](#requirements)  
- [Installation](#installation)  
- [Usage](#usage)  
- [Supported FTP Commands](#supported-ftp-commands)  
- [Logging](#logging)  
- [Contributing](#contributing)  
- [License](#license)  

---

## Features

- **Control Connection:** Listens on `127.0.0.1:2121` for FTP commands over TCP.  
- **Authentication:** Supports `USER`, `PASS`, and `LOGOUT` commands with hardcoded credentials (`alice`, `bob`, `admin`).  
- **File Operations:**  
  - `STOR`: Upload files from client to server.  
  - `RETR`: Download files from server to client.  
  - `LIST`: Retrieve directory listings.  
- **Data Connections:**  
  - `PORT`: Active mode, specifying client IP and port (format: `ip::port`, e.g., `127.0.0.1::2122`). Includes IP validation for security.  
  - `PASV`: Passive mode, server listens on dynamic ports (`2122–2222`) and informs the client.  
- **Directory Navigation:**  
  - `CWD`: Change working directory.  
  - `PWD`: Print working directory.  
- **Session Management:** `QUIT` command for graceful disconnect.  
- **Concurrency:** Handles multiple clients concurrently using Rust’s threading model (`std::thread`) and thread-safe shared state (`Arc<Mutex<>>`).  
- **Logging:** Detailed connection and command logs via the [`log`](https://crates.io/crates/log) crate.  

---

## Project Structure

| File / Module         | Description                                                |
|----------------------|------------------------------------------------------------|
| `main.rs`            | Initializes logging and starts the FTP server.             |
| `server.rs`          | Core FTP server implementation; manages clients & threads. |
| `client.rs`          | Defines client state and authentication data structures.   |
| `client_handler.rs`  | Handles incoming client commands and connection lifecycle. |
| `command.rs`         | Parses FTP commands and defines command-related enums.     |
| `handlers.rs`        | Implements FTP command handlers and logic coordination.    |
| `auth.rs`            | Authentication logic with user credential validation.      |
| `channel_registry.rs`| Manages data channel listeners and connections.            |
| `data_channel.rs`    | Accepts and manages FTP data connections (active/passive). |
| `file_transfer.rs`   | Handles file upload/download operations.                    |
| `lib.rs`             | Library root and module exports.                            |

---

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

| Command  | Description                                                                                 |
| -------- | ------------------------------------------------------------------------------------------- |
| `USER`   | Specify username                                                                            |
| `PASS`   | Specify password                                                                            |
| `QUIT`   | Disconnect from server                                                                      |
| `LOGOUT` | Log out current user                                                                        |
| `LIST`   | Retrieve directory listing                                                                  |
| `PWD`    | Print working directory                                                                     |
| `CWD`    | Change working directory                                                                    |
| `RETR`   | Download/retrieve a file                                                                    |
| `STOR`   | Upload/store a file                                                                         |
| `DEL`    | Delete a file                                                                               |
| `PORT`   | Enter active mode: specify client IP and port for server to connect to (format: `ip::port`) |
| `PASV`   | Enter passive mode: server opens listening port, client connects for data transfer          |


## License

MIT License. See [LICENSE](LICENSE) for details.

