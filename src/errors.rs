#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    EmptyCommand,
    WrongNumberOfArguments,
    UnknownCommand,
}

impl ParseError {
    pub fn response(&self) -> &'static str {
        match self {
            ParseError::EmptyCommand => "ERR empty command",
            ParseError::WrongNumberOfArguments => "ERR wrong number of arguments",
            ParseError::UnknownCommand => "ERR unknown command",
        }
    }
}
