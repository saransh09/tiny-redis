use std::collections::HashMap;

use std::time::{Duration, Instant};

use crate::command::Command;

#[derive(Debug, Default)]
pub struct Entry {
    value: String,
    expires_at: Option<Instant>,
}

#[derive(Debug, Default)]
pub struct Store {
    data: HashMap<String, Entry>,
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
                self.data.insert(
                    key,
                    Entry {
                        value,
                        expires_at: None,
                    },
                );
                "OK".to_string()
            }

            Command::Get { key } => {
                self.remove_if_expired(&key);

                self.data
                    .get(&key)
                    .map(|entry| entry.value.clone())
                    .unwrap_or_else(|| "(nil)".to_string())
            }
            Command::Del { key } => {
                if self.data.remove(&key).is_some() {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }

            Command::Exists { key } => {
                self.remove_if_expired(&key);

                if self.data.contains_key(&key) {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }

            Command::Keys => {
                self.cleanup_expired();

                if self.data.is_empty() {
                    "(empty)".to_string()
                } else {
                    self.data.keys().cloned().collect::<Vec<_>>().join(" ")
                }
            }

            Command::FlushAll => {
                self.data.clear();
                "OK".to_string()
            }

            Command::SetEx {
                key,
                seconds,
                value,
            } => {
                self.data.insert(
                    key,
                    Entry {
                        value,
                        expires_at: Some(Instant::now() + Duration::from_secs(seconds)),
                    },
                );
                "OK".to_string()
            }

            Command::Ttl { key } => {
                self.remove_if_expired(&key);

                match self.data.get(&key) {
                    None => "-2".to_string(),
                    Some(entry) => match entry.expires_at {
                        None => "-1".to_string(),
                        Some(expires_at) => {
                            let now = Instant::now();

                            if now > expires_at {
                                self.data.remove(&key);
                                "-2".to_string()
                            } else {
                                let remaining = expires_at.duration_since(now).as_secs();
                                remaining.to_string()
                            }
                        }
                    },
                }
            }
        }
    }

    fn is_expired(entry: &Entry) -> bool {
        match entry.expires_at {
            Some(expires_at) => Instant::now() >= expires_at,
            None => false,
        }
    }

    fn remove_if_expired(&mut self, key: &str) {
        let expired = self.data.get(key).map(Self::is_expired).unwrap_or(false);
        if expired {
            self.data.remove(key);
        }
    }

    pub fn cleanup_expired(&mut self) {
        let now = Instant::now();
        self.data.retain(|_, entry| {
            entry
                .expires_at
                .map(|expires_at| now < expires_at)
                .unwrap_or(true)
        });
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

    #[test]
    fn keys_returns_existing_keys() {
        let mut store = Store::new();

        store.execute(Command::Set {
            key: "name".to_string(),
            value: "alice".to_string(),
        });

        let response = store.execute(Command::Keys);

        assert_eq!(response, "name");
    }

    #[test]
    fn keys_returns_multiple_existing_keys() {
        let mut store = Store::new();

        store.execute(Command::Set {
            key: "a".to_string(),
            value: "1".to_string(),
        });

        store.execute(Command::Set {
            key: "b".to_string(),
            value: "2".to_string(),
        });

        let response = store.execute(Command::Keys);

        let mut keys: Vec<&str> = response.split_whitespace().collect();
        keys.sort();

        assert_eq!(keys, vec!["a", "b"]);
    }

    #[test]
    fn keys_returns_empty_when_store_is_empty() {
        let mut store = Store::new();

        let response = store.execute(Command::Keys);

        assert_eq!(response, "(empty)");
    }

    #[test]
    fn flushall_clears_all_keys() {
        let mut store = Store::new();

        store.execute(Command::Set {
            key: "name".to_string(),
            value: "alice".to_string(),
        });

        let response = store.execute(Command::FlushAll);

        assert_eq!(response, "OK");

        let get_response = store.execute(Command::Get {
            key: "name".to_string(),
        });

        assert_eq!(get_response, "(nil)");
    }

    #[test]
    fn set_has_no_ttl() {
        let mut store = Store::new();

        store.execute(Command::Set {
            key: "name".to_string(),
            value: "alice".to_string(),
        });

        let response = store.execute(Command::Ttl {
            key: "name".to_string(),
        });

        assert_eq!(response, "-1");
    }

    #[test]
    fn ttl_missing_key_returns_minus_two() {
        let mut store = Store::new();

        let response = store.execute(Command::Ttl {
            key: "missing".to_string(),
        });

        assert_eq!(response, "-2");
    }

    #[test]
    fn setex_stores_value_temporarily() {
        let mut store = Store::new();

        let response = store.execute(Command::SetEx {
            key: "session".to_string(),
            seconds: 1,
            value: "abc".to_string(),
        });

        assert_eq!(response, "OK");

        let response = store.execute(Command::Get {
            key: "session".to_string(),
        });

        assert_eq!(response, "abc");
    }

    #[test]
    fn setex_key_expires() {
        let mut store = Store::new();

        store.execute(Command::SetEx {
            key: "session".to_string(),
            seconds: 1,
            value: "abc".to_string(),
        });

        std::thread::sleep(Duration::from_millis(1100));

        let response = store.execute(Command::Get {
            key: "session".to_string(),
        });

        assert_eq!(response, "(nil)");
    }
}
