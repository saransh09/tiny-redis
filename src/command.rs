use crate::errors::ParseError;

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Ping,
    Set {
        key: String,
        value: String,
    },
    Get {
        key: String,
    },
    Del {
        key: String,
    },
    Exists {
        key: String,
    },
    Keys,
    FlushAll,
    SetEx {
        key: String,
        seconds: u64,
        value: String,
    },
    Ttl {
        key: String,
    },
}

impl Command {
    pub fn parse(input: &str) -> Result<Command, ParseError> {
        let parts: Vec<&str> = input.trim().split_whitespace().collect();

        if parts.is_empty() {
            return Err(ParseError::EmptyCommand);
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

            "KEYS" if parts.len() == 1 => Ok(Command::Keys),

            "FLUSHALL" if parts.len() == 1 => Ok(Command::FlushAll),

            "SETEX" if parts.len() == 4 => {
                let seconds = parts[2]
                    .parse::<u64>()
                    .map_err(|_| ParseError::InvalidInteger)?;

                Ok(Command::SetEx {
                    key: parts[1].to_string(),
                    seconds,
                    value: parts[3].to_string(),
                })
            }

            "TTL" if parts.len() == 2 => Ok(Command::Ttl {
                key: parts[1].to_string(),
            }),

            "PING" | "SET" | "GET" | "DEL" | "EXISTS" | "KEYS" | "FLUSHALL" | "SETEX" | "TTL" => {
                Err(ParseError::WrongNumberOfArguments)
            }

            _ => Err(ParseError::UnknownCommand),
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
            Err(ParseError::UnknownCommand)
        );
    }

    #[test]
    fn rejects_wrong_argument_count() {
        assert_eq!(
            Command::parse("GET"),
            Err(ParseError::WrongNumberOfArguments)
        );
    }

    #[test]
    fn parses_keys() {
        assert_eq!(Command::parse("KEYS"), Ok(Command::Keys));
    }

    #[test]
    fn parses_flushall() {
        assert_eq!(Command::parse("FLUSHALL"), Ok(Command::FlushAll));
    }

    #[test]
    fn rejects_keys_with_arguments() {
        assert_eq!(
            Command::parse("KEYS extra"),
            Err(ParseError::WrongNumberOfArguments)
        );
    }

    #[test]
    fn rejects_flushall_with_arguments() {
        assert_eq!(
            Command::parse("FLUSHALL extra"),
            Err(ParseError::WrongNumberOfArguments)
        );
    }

    #[test]
    fn parses_setex() {
        assert_eq!(
            Command::parse("SETEX session 10 abc"),
            Ok(Command::SetEx {
                key: "session".to_string(),
                seconds: 10,
                value: "abc".to_string(),
            })
        );
    }

    #[test]
    fn parses_ttl() {
        assert_eq!(
            Command::parse("TTL session"),
            Ok(Command::Ttl {
                key: "session".to_string(),
            })
        );
    }

    #[test]
    fn rejects_setex_with_invalid_integer() {
        assert_eq!(
            Command::parse("SETEX session banana abc"),
            Err(ParseError::InvalidInteger)
        );
    }
}
