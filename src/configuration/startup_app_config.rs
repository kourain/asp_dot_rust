use std::{collections::HashMap, env, str::FromStr};

use crate::logging::LOGGER;

/// Recognized token forms, parsed left to right in a single pass:
/// - `--key=value`
/// - `--key value`   (consumes the next token unless it also starts with `--`)
/// - `--flag`        (boolean flag with no value, stored as `"true"`)
#[derive(Default, Debug, Clone)]
pub struct StartupArgsConfiguration {
    values: HashMap<String, String>,
}

impl StartupArgsConfiguration {
    pub fn with_args(&mut self) {
        let args: Vec<String> = env::args().skip(1).collect();
        self.parse(args);
        LOGGER::info(format!("Parsed {} command-line argument(s)", self.len()));
    }
    /// Parse an arbitrary argument list. Exposed mainly so callers and tests
    /// can parse a custom source; production code should generally use
    /// `ApplicationBuilder::with_args`.
    pub fn parse<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut iter = args.into_iter().map(Into::into).peekable();
        self.values = HashMap::with_capacity(iter.size_hint().0);
        let values = &mut self.values;
        while let Some(token) = iter.next() {
            let Some(key) = token.strip_prefix("--") else {
                continue; // ignore positional / non `--` tokens
            };
            if key.is_empty() {
                continue;
            }
            if let Some((k, v)) = key.split_once('=') {
                values.insert(k.to_string(), v.to_string());
                continue;
            }
            match iter.peek() {
                Some(next) if !next.starts_with("--") => {
                    values.insert(key.to_string(), iter.next().unwrap());
                }
                _ => {
                    values.insert(key.to_string(), "true".to_string());
                }
            }
        }
        self
    }

    /// Returns the raw string value for `key`, if present.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    /// Returns the value for `key` parsed as `T`, or `None` if the key is
    /// missing or fails to parse.
    pub fn get_parsed<T: FromStr>(&self, key: &str) -> Option<T> {
        self.values.get(key)?.parse().ok()
    }

    pub fn contains(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}
