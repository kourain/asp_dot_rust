use crate::{ApplicationBuilder, configuration::StartupArgsConfiguration, dependency_injection::CfgRequire};

impl ApplicationBuilder {
    /// Parse `std::env::args()` (skipping the binary name) and apply them to
    /// this run. Recognized flags (eg: `--ip`, `--http-port`, `--https-port`) are
    /// applied directly to the builder; the full parsed set is stored in
    /// `ConfigurationService` as `StartupAppConfiguration` so any service can
    /// read custom flags via `CfgOption<StartupAppConfiguration>`.
    pub(crate) fn load_data_from_args(&mut self) -> &mut Self {
        let new_cli = self.configuration.get::<StartupArgsConfiguration>().capture().unwrap();

        self.set_ip(&new_cli);
        self.set_http_port(&new_cli);
        self.set_https_port(&new_cli);
        self
    }

    fn set_ip(&mut self, cli: &CfgRequire<StartupArgsConfiguration>) {
        if let Some(ip) = cli.get("ip") {
            self.with_ip(ip);
        }
    }
    fn set_http_port(&mut self, cli: &CfgRequire<StartupArgsConfiguration>) {
        if let Some(port) = cli.get_parsed::<u16>("http-port") {
            self.with_http_port(port);
        }
    }
    fn set_https_port(&mut self, cli: &CfgRequire<StartupArgsConfiguration>) {
        if let Some(port) = cli.get_parsed::<u16>("https-port") {
            self.with_https_port(port);
        }
    }
}
