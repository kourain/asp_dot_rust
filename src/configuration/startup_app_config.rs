use std::{collections::HashMap, env, ops::Deref, str::FromStr};

use crate::{application::ApplicationBuilder, logging::LOGGER};

/// Recognized token forms, parsed left to right in a single pass:
/// - `--key=value`
/// - `--key value`   (consumes the next token unless it also starts with `--`)
/// - `--flag`        (boolean flag with no value, stored as `"true"`)
#[derive(Default, Debug, Clone)]
pub struct StartupAppConfiguration {
    values: HashMap<String, String>,
}

impl StartupAppConfiguration {
    /// Parse an arbitrary argument list. Exposed mainly so callers and tests
    /// can parse a custom source; production code should generally use
    /// `ApplicationBuilder::with_args`.
    pub fn parse<I, S>(&mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut iter = args.into_iter().map(Into::into).peekable();
        let mut values = HashMap::with_capacity(iter.size_hint().0);

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
        Self { values }
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

impl ApplicationBuilder {
    /// Parse `std::env::args()` (skipping the binary name) and apply them to
    /// this run. Recognized flags (eg: `--ip`, `--http-port`, `--https-port`) are
    /// applied directly to the builder; the full parsed set is stored in
    /// `ConfigurationService` as `StartupAppConfiguration` so any service can
    /// read custom flags via `Option<Arc<StartupAppConfiguration>>`.
    pub(crate) fn with_args(&mut self) -> &mut Self {
        let args: Vec<String> = env::args().skip(1).collect();
        if !self.configuration.contains::<StartupAppConfiguration>() {
            self.configuration.insert(StartupAppConfiguration::default());
        }
    
        let old_cli = self.configuration.get::<StartupAppConfiguration>().unwrap();
        let new_cli = old_cli.deref().clone().parse(args);

        self.set_ip(&new_cli);
        self.set_http_port(&new_cli);
        self.set_https_port(&new_cli);

        LOGGER::info(format!("Parsed {} command-line argument(s)", new_cli.len()));
        self.configuration.insert(new_cli);
        self
    }
    fn set_ip(&mut self, cli: &StartupAppConfiguration) {
        if let Some(ip) = cli.get("ip") {
            self.with_ip(ip);
        }
    }
    fn set_http_port(&mut self, cli: &StartupAppConfiguration) {
        if let Some(port) = cli.get_parsed::<u16>("http-port") {
            self.with_http_port(port);
        }
    }
    fn set_https_port(&mut self, cli: &StartupAppConfiguration) {
        if let Some(port) = cli.get_parsed::<u16>("https-port") {
            if let Some(cert_path) = cli.get("tls-cert"){
                if let Some(key_path) = cli.get("tls-key"){
                    self.with_https_port(port, cert_path, key_path);
                }
            }
        }
    }
}
