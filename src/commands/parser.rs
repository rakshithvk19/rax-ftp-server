// Command enum to represent FTP commands
#[derive(Debug, PartialEq)]
pub enum Command {
    Quit,
    List,
    Logout,
    Pwd,
    Cwd(String),
    User(String),
    Pass(String),
    Retr(String),
    Stor(String),
    Unknown(String),
}

#[derive(Debug, PartialEq)]
pub enum CommandResult {
    Quit,
    Continue,
    Stor,
}

// Parse raw command string into Command enum
pub fn parse_command(raw: &str) -> Command {
    let trimmed = raw.trim();
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let cmd = parts.next().unwrap_or("").to_ascii_uppercase();
    let arg = parts.next().unwrap_or("").trim();

    match cmd.as_str() {
        "QUIT" | "Q" => Command::Quit,
        "LIST" => Command::List,
        "LOGOUT" => Command::Logout,
        "PWD" => Command::Pwd,
        "CWD" => Command::Cwd(arg.to_string()),
        "USER" => Command::User(arg.to_string()),
        "PASS" => Command::Pass(arg.to_string()),
        "RETR" => Command::Retr(arg.to_string()),
        "STOR" => Command::Stor(arg.to_string()),
        _ => Command::Unknown(trimmed.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_commands() {
        assert_eq!(parse_command("QUIT"), Command::Quit);
        assert_eq!(parse_command("Q"), Command::Quit);
        assert_eq!(parse_command("LIST"), Command::List);
        assert_eq!(parse_command("LOGOUT"), Command::Logout);
        assert_eq!(parse_command("PWD"), Command::Pwd);
    }

    #[test]
    fn test_parse_commands_with_args() {
        assert_eq!(
            parse_command("CWD /some/path"),
            Command::Cwd("/some/path".to_string())
        );
        assert_eq!(
            parse_command("USER username"),
            Command::User("username".to_string())
        );
        assert_eq!(
            parse_command("PASS secret"),
            Command::Pass("secret".to_string())
        );
        assert_eq!(
            parse_command("RETR file.txt"),
            Command::Retr("file.txt".to_string())
        );
        assert_eq!(
            parse_command("STOR upload.txt"),
            Command::Stor("upload.txt".to_string())
        );
    }

    #[test]
    fn test_parse_with_whitespace() {
        assert_eq!(parse_command("  QUIT  "), Command::Quit);
        assert_eq!(parse_command("LIST    "), Command::List);
        assert_eq!(
            parse_command("USER  john  "),
            Command::User("john".to_string())
        );
    }

    #[test]
    fn test_unknown_commands() {
        assert_eq!(
            parse_command("INVALID"),
            Command::Unknown("INVALID".to_string())
        );
        assert_eq!(
            parse_command("FOO bar"),
            Command::Unknown("FOO bar".to_string())
        );
        assert_eq!(parse_command(""), Command::Unknown("".to_string()));
    }
}
