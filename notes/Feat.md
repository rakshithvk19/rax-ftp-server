1. user client auth feature
    - implement username/password authentication
    - create user database/storage
    - add session management
    - implement login/logout endpoints
    - add password hashing
    - handle authentication errors

2. implement complete response codes
    - map all FTP standard response codes
    - implement success codes (2xx)
    - implement authentication codes (3xx)
    - implement error codes (4xx, 5xx)
    - add proper error messages
    - document response code meanings


BUGS
1. Cargo should stop on quit use TcpStream::Shutdown
2. //Check if input is file or dir in RETR
3. Use ZKPs for Auth ZK lib in rust
4. throw error when there is an error while streaming mid way.
5. user can be changed when session is going on -- fatal bug 

src/
├── auth.rs
├── client/
│   ├── client_state.rs
│   ├── client_connection.rs
│   └── mod.rs
├── commands/
│   ├── file_ops.rs
│   ├── handlers.rs
│   ├── parser.rs
│   └── mod.rs
├── data_channel.rs
├── errors.rs
├── file_ops.rs
├── logging.rs
├── network_utils.rs
├── config.rs
├── server.rs
├── main.rs
└── lib.rs
