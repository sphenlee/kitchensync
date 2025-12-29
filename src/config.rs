use std::fs::{self, read_to_string};

use serde::Deserialize;

use crate::{KResult, KCONFIG};

#[derive(Deserialize, Default, Debug)]
pub struct Destination {
    pub name: String,
    pub target: String,
}

#[derive(Deserialize, Default, Debug)]
pub struct Config {
    pub destination: Vec<Destination>,
}

pub fn load() -> KResult<Config> {
    if !fs::exists(KCONFIG)? {
        return Ok(Config::default());
    }
    let raw = read_to_string(KCONFIG)?;

    let config: Config = toml::from_str(&raw)?;

    Ok(config)
}
