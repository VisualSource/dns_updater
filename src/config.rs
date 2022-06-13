use std::{path::PathBuf, collections::HashMap};
use tokio::fs;

use serde::{Deserialize,Serialize };

#[derive(Debug,Deserialize,Serialize)]
pub struct Domain {
    pub username: String,
    pub psd: String,
    pub domain: String
}

#[derive(Debug,Deserialize,Serialize, Default)]
pub struct Config {
    pub domains: Vec<Domain>,
    pub debug: bool
}

#[derive(Debug,Deserialize,Serialize)]
pub struct CachedDns {
    pub ip: String,
    pub changed: String
}

impl CachedDns {
    pub fn new(ip: String, changed: String) -> Self {
        Self { ip, changed }
    }
}

pub async fn load_config() -> Result<Config,String> { 
    let config = PathBuf::from("./config.json");

    if !config.exists() {
        let default_config = Config::default();
        let config_str = match serde_json::to_string(&default_config) {
            Ok(value) => value,
            Err(err) => return Err(err.to_string())
        };

        if let Err(err) = fs::write(config.clone(), config_str).await {
            return Err(err.to_string());
        }

        return Ok(default_config);
    }

    match fs::read_to_string(config).await {
        Ok(value) => {
            match serde_json::from_str::<Config>(&value) {
                Ok(data) => Ok(data),
                Err(err) => Err(err.to_string())
            }
        }
        Err(err) => Err(err.to_string())
    }
}

pub async fn write_dns_cache(data: HashMap<String,CachedDns>) -> Result<(),String> {
    let cache_file = PathBuf::from("./dns_cache.json");
    
    let cache_str = match serde_json::to_string(&data) {
        Ok(value) => value,
        Err(err) => return Err(err.to_string())
    };

    if let Err(err) = fs::write(cache_file, cache_str).await {
        return Err(err.to_string());
    }

    Ok(())
}

pub async fn read_dns_cache() -> Result<HashMap<String,CachedDns>,String> {
    let cache_file = PathBuf::from("./dns_cache.json");

    if !cache_file.exists() {  
        if let Err(err) = fs::write(cache_file.clone(), b"{}").await {
            return Err(err.to_string());
        }

        return Ok(HashMap::new());
    }

    match fs::read_to_string(cache_file).await {
        Ok(value) => {
            match serde_json::from_str::<HashMap<String,CachedDns>>(&value) {
                Ok(data) => Ok(data),
                Err(err) => Err(err.to_string())
            }
        },
        Err(err) => Err(err.to_string())   
    }
}

pub async fn write_locked_dns(data: Vec<String>) -> Result<(),String> {
    let cache_file = PathBuf::from("./dns_errored.json");
    
    let cache_str = match serde_json::to_string(&data) {
        Ok(value) => value,
        Err(err) => return Err(err.to_string())
    };

    if let Err(err) = fs::write(cache_file, cache_str).await {
        return Err(err.to_string());
    }

    Ok(())
}

pub async fn read_locked_dns() ->  Result<Vec<String>,String> {
    let cache_file = PathBuf::from("./dns_errored.json");

    if !cache_file.exists() {  
        return Ok(vec![]);
    }

    match fs::read_to_string(cache_file).await {
        Ok(value) => {
            match serde_json::from_str::<Vec<String>>(&value) {
                Ok(data) => Ok(data),
                Err(err) => Err(err.to_string())
            }
        },
        Err(err) => Err(err.to_string())   
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_dns_cache(){
        match read_dns_cache().await {
            Ok(value) => println!("{:#?}",value),
            Err(err) => {
                eprintln!("{}",err);
                panic!();
            }
        }
    }

    #[tokio::test]
    async fn test_load_config() {
        match load_config().await {
            Ok(value) => println!("{:#?}",value),
            Err(err) => {
                eprintln!("{}",err);
                panic!()
            }
        }
    }
}