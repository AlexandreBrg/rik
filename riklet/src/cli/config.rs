use clap::Parser;
use cri::container::RuncConfiguration;
use oci::image_manager::ImageManagerConfiguration;
use oci::skopeo::SkopeoConfiguration;
use oci::umoci::UmociConfiguration;
use serde::{Deserialize, Serialize};
use shared::utils::{create_directory_if_not_exists, create_file_with_parent_folders};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::CliConfiguration;
use crate::constants::DEFAULT_COMMAND_TIMEOUT;
use snafu::Snafu;
use tracing::{event, Level};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Unable to load the configuration file. Error {}", source))]
    Load { source: std::io::Error },
    #[snafu(display("Unable to parse the configuration file. Error {}", source))]
    Parse { source: toml::de::Error },
    #[snafu(display("Unable to encode the configuration in TOML format. Error {}", source))]
    TomlEncode { source: toml::ser::Error },
    #[snafu(display("Unable to create the configuration. Error {}", source))]
    ConfigFileCreation { source: std::io::Error },
    #[snafu(display(
        "An error occured when trying to write the configuration. Error {}",
        source
    ))]
    ConfigFileWrite { source: std::io::Error },
    #[snafu(display("An error occured when trying to create the {} directory. Error {}", path.display(), source))]
    CreateDirectory {
        source: std::io::Error,
        path: PathBuf,
    },
    #[snafu(display("Unable to parse the IP. Error {}", source))]
    InvalidIp { source: std::net::AddrParseError },
}

#[derive(Deserialize, Debug, Serialize, PartialEq, Clone)]
pub struct Configuration {
    pub master_ip: String,
    pub log_level: String,
    pub runner: RuncConfiguration,
    pub manager: ImageManagerConfiguration,
}

impl Configuration {
    fn get_cli_args() -> CliConfiguration {
        CliConfiguration::parse()
    }

    /// Create the configuration file and store the default config into it
    fn create(
        path: &Path,
        configuration: &Configuration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        event!(Level::INFO, "No configuration file found at {}. Creating a new configuration file with the default configuration.", path.display());
        let toml = toml::to_string(configuration).map_err(|source| Error::TomlEncode { source })?;

        let mut file = create_file_with_parent_folders(path)
            .map_err(|source| Error::ConfigFileCreation { source })?;

        file.write_all(&toml.into_bytes())
            .map_err(|source| Error::ConfigFileWrite { source })?;

        Ok(())
    }

    /// Read the configuration file from the path provided.
    fn read(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        event!(
            Level::DEBUG,
            "Reading configuration from file {}",
            path.display()
        );
        let contents = std::fs::read(path).map_err(|source| Error::Load { source })?;

        Ok(toml::from_slice(&contents).map_err(|source| Error::Parse { source })?)
    }

    /// Load the configuration file
    /// If not exists, create it and return the default configuration
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        event!(Level::DEBUG, "Loading configuration");
        let opts = Configuration::get_cli_args();

        let path = PathBuf::from(&opts.config_file);

        let mut configuration = Configuration::default();

        if !path.exists() {
            configuration.override_config(&opts);
            Configuration::create(&path, &configuration)?;
        } else {
            configuration = Configuration::read(&path)?;
            if opts.override_config {
                configuration.override_config(&opts);
            }
        };

        event!(
            Level::DEBUG,
            "Loaded configuration from file {}",
            path.display()
        );

        configuration.bootstrap()?;

        Ok(configuration)
    }

    /// Override the configuration instance
    pub fn override_config(&mut self, opts: &CliConfiguration) {
        if let Some(master_ip) = opts.master_ip.clone() {
            self.master_ip = format!("http://{}", master_ip);
        }
    }

    /// Create all directories and files used by Riklet to work properly
    pub fn bootstrap(&self) -> Result<(), Error> {
        event!(
            Level::DEBUG,
            "Create all directories and files used by Riklet to work properly"
        );
        let bundles_dir = self.manager.oci_manager.bundles_directory.clone();
        let images_dir = self.manager.image_puller.images_directory.clone();

        create_directory_if_not_exists(&bundles_dir).map_err(|source| Error::CreateDirectory {
            source,
            path: bundles_dir.unwrap(),
        })?;

        create_directory_if_not_exists(&images_dir).map_err(|source| Error::CreateDirectory {
            source,
            path: images_dir.unwrap(),
        })?;

        Ok(())
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            master_ip: String::from("http://127.0.0.1:4995"),
            log_level: String::from("info"),
            runner: RuncConfiguration {
                debug: false,
                rootless: false,
                root: None,
                command: None,
                timeout: Some(Duration::from_millis(DEFAULT_COMMAND_TIMEOUT)),
            },
            manager: ImageManagerConfiguration {
                image_puller: SkopeoConfiguration {
                    images_directory: Some(PathBuf::from("/var/lib/riklet/images")),
                    timeout: Some(Duration::from_millis(DEFAULT_COMMAND_TIMEOUT)),
                    debug: false,
                    insecure_policy: false,
                    ..Default::default()
                },
                oci_manager: UmociConfiguration {
                    timeout: Some(Duration::from_millis(DEFAULT_COMMAND_TIMEOUT)),
                    bundles_directory: Some(PathBuf::from("/var/lib/riklet/bundles")),
                    debug: false,
                    ..Default::default()
                },
            },
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::config::Configuration;
//     use std::path::PathBuf;
//     use uuid::Uuid;
//
//     #[test]
//     fn test_it_load_configuration() {
//         let config_id = format!("riklet-{}.toml", Uuid::new_v4());
//         let config_path = std::env::temp_dir().join(PathBuf::from(config_id));
//
//         let configuration = Configuration::load().expect("Failed to load configuration");
//
//         assert_eq!(configuration, Configuration::default())
//     }
//
//     #[test]
//     fn test_it_create_configuration() {
//         let config_id = format!("riklet-{}.toml", Uuid::new_v4());
//         let config_path = std::env::temp_dir().join(PathBuf::from(config_id));
//
//         assert!(!&config_path.exists());
//
//         let configuration = Configuration::load().expect("Failed to load configuration");
//
//         assert!(&config_path.exists());
//         assert_eq!(configuration, Configuration::default())
//     }
// }
