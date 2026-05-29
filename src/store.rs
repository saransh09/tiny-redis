use std::collections::HashMap;

use crate::command::Command;

#[derive(Debug, Default)]
pub struct Store {
    data: HashMap<String, String>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn execute(&mut self, command: Command) -> String {
        match command {
            Command::Ping => "PONG".to_string(),

            Command::Set { key, value } => {
                self.data.insert(key, value);
                "OK".to_string()
            }

            Command::Get { key } => self
                .data
                .get(&key)
                .cloned()
                .unwrap_or_else(|| "(nil)".to_string()),

            Command::Del { key } => {
                if self.data.remove(&key).is_some() {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }

            Command::Exists { key } => {
                if self.data.contains_key(&key) {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_returns_pong() {
        let mut store = Store::new();

        let response = store.execute(Command::Ping);

        assert_eq!(response, "PONG");
    }

    #[test]
    fn set_then_get_returns_value() {
        let mut store = Store::new();

        store.execute(Command::Set {
            key: "name".to_string(),
            value: "alice".to_string(),
        });

        let response = store.execute(Command::Get {
            key: "name".to_string(),
        });

        assert_eq!(response, "alice");
    }

    #[test]
    fn get_missing_key_returns_nil() {
        let mut store = Store::new();

        let response = store.execute(Command::Get {
            key: "missing".to_string(),
        });

        assert_eq!(response, "(nil)");
    }

    #[test]
    fn del_existing_key_returns_one() {
        let mut store = Store::new();

        store.execute(Command::Set {
            key: "name".to_string(),
            value: "alice".to_string(),
        });

        let response = store.execute(Command::Del {
            key: "name".to_string(),
        });

        assert_eq!(response, "1");
    }

    #[test]
    fn exists_returns_one_for_existing_key() {
        let mut store = Store::new();

        store.execute(Command::Set {
            key: "name".to_string(),
            value: "alice".to_string(),
        });

        let response = store.execute(Command::Exists {
            key: "name".to_string(),
        });

        assert_eq!(response, "1");
    }
}
