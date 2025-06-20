// use std::fs::{self, File};
// use std::io::{Read, Write};
// use std::net::TcpStream;
// use std::path::Path;
// use std::thread;
// use std::time::Duration;

// use rax_ftp_server::Server;

// // Helper to connect to the server
// fn connect() -> TcpStream {
//     let mut attempts = 5;
//     loop {
//         match TcpStream::connect("127.0.0.1:2121") {
//             Ok(stream) => return stream,
//             Err(_) if attempts > 0 => {
//                 thread::sleep(Duration::from_millis(100));
//                 attempts -= 1;
//             }
//             Err(e) => panic!("Failed to connect: {}", e),
//         }
//     }
// }

// // Helper to send command and read response
// fn send_command(stream: &mut TcpStream, command: &str) -> String {
//     stream
//         .write_all(format!("{}\r\n", command).as_bytes())
//         .unwrap();
//     stream.flush().unwrap();
//     let mut buffer = [0; 1024];
//     let bytes_read = stream.read(&mut buffer).unwrap();
//     String::from_utf8_lossy(&buffer[..bytes_read]).to_string()
// }

// // Helper to setup test environment
// fn setup_test_env() {
//     // Create temporary directory
//     fs::create_dir_all("test_temp").unwrap();
//     // Create a test file
//     File::create("test_temp/test.txt").unwrap();
//     // Create a subdirectory
//     fs::create_dir_all("test_temp/dir1").unwrap();
//     // Set working directory to test_temp
//     std::env::set_current_dir("test_temp").unwrap();
// }

// // Helper to cleanup test environment
// fn cleanup_test_env() {
//     std::env::set_current_dir("..").unwrap();
//     fs::remove_dir_all("test_temp").unwrap();
// }

// // Start server in a separate thread
// fn start_test_server() {
//     thread::spawn(|| {
//         let _server = Server::new();
//     });
//     // Wait for server to start
//     thread::sleep(Duration::from_millis(200));
// }

// #[test]
// fn test_initial_connection() {
//     start_test_server();
//     let mut stream = connect();
//     let mut buffer = [0; 1024];
//     let bytes_read = stream.read(&mut buffer).unwrap();
//     let response = String::from_utf8_lossy(&buffer[..bytes_read]);
//     assert!(response.starts_with("220 Welcome"));
// }

// #[test]
// fn test_user_command() {
//     start_test_server();
//     let mut stream = connect();
//     let response = send_command(&mut stream, "USER user");
//     assert_eq!(response.trim(), "331 Password required");
//     let response = send_command(&mut stream, "USER baduser");
//     assert_eq!(response.trim(), "530 Invalid username");
// }

// #[test]
// fn test_pass_command() {
//     start_test_server();
//     let mut stream = connect();
//     send_command(&mut stream, "USER user");
//     let response = send_command(&mut stream, "PASS pass");
//     assert_eq!(response.trim(), "230 Login successful");
//     send_command(&mut stream, "USER user");
//     let response = send_command(&mut stream, "PASS badpass");
//     assert_eq!(response.trim(), "530 Invalid password");
//     let mut stream = connect();
//     let response = send_command(&mut stream, "PASS pass");
//     assert_eq!(response.trim(), "530 Please enter the username first");
// }

// #[test]
// fn test_quit_command() {
//     start_test_server();
//     let mut stream = connect();
//     let response = send_command(&mut stream, "QUIT");
//     assert_eq!(response.trim(), "221 Goodbye");
//     // Verify connection closed
//     let result = stream.write_all(b"NOOP\r\n");
//     assert!(result.is_err());
// }

// #[test]
// fn test_list_command() {
//     setup_test_env();
//     start_test_server();
//     let mut stream = connect();
//     send_command(&mut stream, "USER user");
//     send_command(&mut stream, "PASS pass");
//     let response = send_command(&mut stream, "LIST");
//     assert!(response.contains("150 Opening data connection"));
//     assert!(response.contains("test.txt"));
//     assert!(response.contains("dir1"));
//     let mut stream = connect();
//     let response = send_command(&mut stream, "LIST");
//     assert_eq!(response.trim(), "530 Not logged in");
//     cleanup_test_env();
// }

// #[test]
// fn test_retr_command() {
//     setup_test_env();
//     start_test_server();
//     let mut stream = connect();
//     send_command(&mut stream, "USER user");
//     send_command(&mut stream, "PASS pass");
//     let response = send_command(&mut stream, "RETR test.txt");
//     assert!(response.contains("150 Opening data connection"));
//     assert!(response.contains("226 Transfer complete"));
//     let response = send_command(&mut stream, "RETR nonexistent.txt");
//     assert_eq!(response.trim(), "550 File not found");
//     let mut stream = connect();
//     let response = send_command(&mut stream, "RETR test.txt");
//     assert_eq!(response.trim(), "530 Not logged in");
//     cleanup_test_env();
// }

// #[test]
// fn test_stor_command() {
//     setup_test_env();
//     start_test_server();

//     let mut stream = connect();

//     send_command(&mut stream, "USER user");
//     send_command(&mut stream, "PASS pass");

//     let response = send_command(&mut stream, "STOR newfile.txt");
//     assert_eq!(response.trim(), "226 Transfer complete");
//     assert!(Path::new("newfile.txt").exists());

//     let response = send_command(&mut stream, "STOR test.txt");
//     assert_eq!(response.trim(), "550 File already exists");

//     let response = send_command(&mut stream, "STOR invalid/../file");
//     assert_eq!(response.trim(), "550 Filename invalid");

//     let mut stream = connect();
//     let response = send_command(&mut stream, "STOR newfile.txt");
//     assert_eq!(response.trim(), "530 Not logged in");

//     cleanup_test_env();
// }

// #[test]
// fn test_logout_command() {
//     start_test_server();
//     let mut stream = connect();
//     send_command(&mut stream, "USER user");
//     send_command(&mut stream, "PASS pass");
//     let response = send_command(&mut stream, "LOGOUT");
//     assert_eq!(response.trim(), "221 Logout successful");
//     let response = send_command(&mut stream, "LIST");
//     assert_eq!(response.trim(), "530 Not logged in");
//     let response = send_command(&mut stream, "LOGOUT");
//     assert_eq!(response.trim(), "530 User Not logged in");
// }

// #[test]
// fn test_unknown_command() {
//     start_test_server();
//     let mut stream = connect();
//     send_command(&mut stream, "USER user");
//     send_command(&mut stream, "PASS pass");
//     let response = send_command(&mut stream, "BADCMD");
//     assert_eq!(response.trim(), "500 Unknown command");
//     let response = send_command(&mut stream, "rax");
//     assert_eq!(response.trim(), "Rax is the best");
//     let mut stream = connect();
//     let response = send_command(&mut stream, "BADCMD");
//     assert_eq!(response.trim(), "530 Not logged in");
// }

// #[test]
// fn test_cwd_command() {
//     setup_test_env();
//     start_test_server();
//     let mut stream = connect();
//     send_command(&mut stream, "USER user");
//     send_command(&mut stream, "PASS pass");
//     let response = send_command(&mut stream, "CWD dir1");
//     assert_eq!(response.trim(), "250 Requested file action okay, completed");
//     let response = send_command(&mut stream, "CWD nonexistent");
//     assert_eq!(response.trim(), "550 Requested action not taken");
//     let response = send_command(&mut stream, "CWD");
//     assert!(response.contains("550") || response.contains("501")); // Depends on implementation
//     let mut stream = connect();
//     let response = send_command(&mut stream, "CWD dir1");
//     assert_eq!(response.trim(), "530 Not logged in");
//     cleanup_test_env();
// }

// #[test]
// fn test_pwd_command() {
//     setup_test_env();
//     start_test_server();
//     let mut stream = connect();
//     send_command(&mut stream, "USER user");
//     send_command(&mut stream, "PASS pass");
//     let response = send_command(&mut stream, "PWD");
//     assert!(response.starts_with("257 \""));
//     assert!(response.contains("test_temp"));
//     send_command(&mut stream, "CWD dir1");
//     let response = send_command(&mut stream, "PWD");
//     assert!(response.contains("dir1"));
//     let mut stream = connect();
//     let response = send_command(&mut stream, "PWD");
//     assert_eq!(response.trim(), "530 Not logged in");
//     cleanup_test_env();
// }

// #[test]
// fn test_command_sequence() {
//     setup_test_env();
//     start_test_server();
//     let mut stream = connect();
//     send_command(&mut stream, "USER user");
//     send_command(&mut stream, "PASS pass");
//     send_command(&mut stream, "CWD dir1");
//     send_command(&mut stream, "PWD");
//     let response = send_command(&mut stream, "STOR newfile.txt");
//     assert_eq!(response.trim(), "226 Transfer complete");
//     assert!(Path::new("dir1/newfile.txt").exists());
//     let response = send_command(&mut stream, "LIST");
//     assert!(response.contains("newfile.txt"));
//     send_command(&mut stream, "LOGOUT");
//     let response = send_command(&mut stream, "LIST");
//     assert_eq!(response.trim(), "530 Not logged in");
//     cleanup_test_env();
// }
