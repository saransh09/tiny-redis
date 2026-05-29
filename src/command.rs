#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Ping,
    Set { key: String, value: String },
    Get { key: String },
    Del { key: String },
    Exists { key: String },
}

impl Command {
    pub fn parse(input: &str) -> Result<Self, String> {
        let parts: Vec<&str> = input.trim().split_whitespace().collect();

        if parts.is_empty() {
            return Err("ERR empty commmand".to_string());
        }

        match parts[0].to_uppercase().as_str() {
            "PING" if parts.len() == 1 => Ok(Command::Ping),
            "SET" if parts.len() == 3 => Ok(Command::Set {
                key: parts[1].to_string(),
                value: parts[2].to_string(),
            }),
            "GET" if parts.len() == 2 => Ok(Command::Get {
                key: parts[1].to_string(),
            }),
            "DEL" if parts.len() == 2 => Ok(Command::Del {
                key: parts[1].to_string(),
            }),
            "EXISTS" if parts.len() == 2 => Ok(Command::Exists {
                key: parts[1].to_string(),
            }),

            "PING" | "SET" | "GET" | "DEL" | "EXISTS" => {
                Err("ERR wrong number of arguments".to_string())
            }

            _ => Err("ERR unknown command".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ping() {
        assert_eq!(Command::parse("PING"), Ok(Command::Ping));
    }

    #[test]
    fn parses_set() {
        assert_eq!(
            Command::parse("SET name alice"),
            Ok(Command::Set {
                key: "name".to_string(),
                value: "alice".to_string(),
            })
        );
    }

    #[test]
    fn parses_get() {
        assert_eq!(
            Command::parse("GET name"),
            Ok(Command::Get {
                key: "name".to_string(),
            })
        );
    }

    #[test]
    fn rejects_unknown_command() {
        assert_eq!(
            Command::parse("BANANA name"),
            Err("ERR unknown command".to_string())
        );
    }

    #[test]
    fn rejects_wrong_argument_count() {
        assert_eq!(
            Command::parse("GET"),
            Err("ERR wrong number of arguments".to_string())
        );
    }
}
