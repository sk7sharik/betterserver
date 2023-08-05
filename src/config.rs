use std::fs;

use log::warn;
use serde::{Serialize, Deserialize};
use lazy_static::lazy_static;

#[derive(Serialize, Deserialize)]
pub(crate) struct ServerConfiguration 
{
    pub count: i32,
    pub tcp_port: i32,
    pub udp_port: i32
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Configuration
{
    pub server: ServerConfiguration,
    pub gui: bool,
    pub debug: bool
}

const DEFAULT_CONFIG: Configuration = Configuration { 
    server: ServerConfiguration {
        count: 1,
        tcp_port: 7606,
        udp_port: 8606
    },

    gui: true,
    debug: true,
};

 lazy_static! {
    pub(crate) static ref CONFIG: Configuration = init_config();
}

fn default_config() -> Configuration
{
    let value = match toml::to_string(&DEFAULT_CONFIG)
    {
        Ok(res) => res,
        Err(err) => {
            println!("Failed to shit: {}", err);
            return DEFAULT_CONFIG
        }
    };

    match fs::write("Config.toml", value.clone())
    {
        Ok(_) => {},
        Err(err) => {
            println!("Failed to save default configuration: {}", err);
        }
    }

    DEFAULT_CONFIG
}

fn init_config() -> Configuration
{
    let result = match fs::read_to_string("Config.toml")
    {
        Ok(res) => res,
        Err(err) => {
            println!("Failed to open config file: {}", err);
            return default_config();
        }
    };
        
    // we only assign once, hence we dont care
    match toml::from_str(&result)
    {
        Ok(result) => result,
        Err(err) => {
            println!("Failed to parase config: {}", err);
            return DEFAULT_CONFIG;
        }
    }
}