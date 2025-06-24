# FTP Server Project Roadmap

## Project Setup and Basic Commands
1. Set up Rust project with `cargo`, add dependencies (`log`, `env_logger`).
2. Implement TCP server in `main.rs` using `TcpListener`.
3. Create modular structure: `server.rs` (client handling), `commands.rs` (command parsing), `client.rs` (client authentication).
4. Implement `USER`, `PASS`, `QUIT` commands with basic client auth (`client.rs`).
5. Send FTP responses (`220`, `230`, `331`, `221`).

## File Commands and Regression Fixes
1. Implement `LIST`, `RETR`, `STOR` commands.
2. Add `LOGOUT` command.

## Directory Navigation
1. Implement `CWD` (change working directory) and `PWD` (print working directory). -- done
2. Enhance testing for all commands.

## Multi-Threading Optimization
1. Optimize server for multiple clients using threads.
2. Ensure thread safety for shared resources.

## Data Connection Support
1. Implement `PORT` command for active mode data connections. -- done
2. Extend `STOR` to receive data over a separate `TcpStream` using `PUT` command. -- done

## Async Refactoring with Tokio
1. Refactor server to use Tokio for async I/O.
2. Support `PASV` for passive mode data connections. -- done

## Additional Commands
1. Implement `DELE` (delete file) -- done, 
2. `MKD` (make directory), `RMD` (remove directory).
2. Enhance error handling.

## Security and Configuration
1. Add basic security (e.g., username/password validation).
2. Support configuration file for server settings (port, root dir).

## Performance and Testing
1. Optimize performance for large files and many clients.
2. Expand test suite with unit and integration tests.

## Documentation and Polish
1. Finalize portfolio-ready documentation.
2. Polish code and UI (logging, responses).
3. Prepare demo for job applications.
