mod config;
mod network;

use futures::StreamExt;
use futures::stream;
use std::time::Duration;
use std::collections::HashMap;
use std::io::ErrorKind;
use std::io::Error;
use std::sync::{ Arc, Mutex };
use chrono::Utc;
use log::{ info, error, warn };
use tokio::time::sleep;
use config::read_locked_dns;
use config::write_dns_cache;
use config::write_locked_dns;
use config::init_logger;
use network::{get_exteral_ip, format_request};
use config::{load_config, read_dns_cache};


static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"),"/",env!("CARGO_PKG_VERSION"),);


#[tokio::main]
async fn main() -> std::io::Result<()> {

    init_logger();
   
    let config = match load_config().await {
        Ok(value) => value,
        Err(err) => {
            error!("Failed to load config file! | {}",err);
            return Err(Error::new(ErrorKind::Other,err))
        }
    };
    
    let ip = match get_exteral_ip(&config).await {
        Ok(value) => value,
        Err(err) => {
            error!("Failed to fetch external ip | {}", err);
            return Err(Error::new(ErrorKind::Other,err));
        }
    };

    info!("Current external IP({})",ip);

    let status_file = include_str!("request.json");
    let status = match serde_json::from_str::<HashMap<String,String>>(status_file) {
        Ok(mut value) => {

            value.insert(format!("good {}",&ip), "The update was successful. You should not attempt another update until your IP address changes.".into());
            value.insert(format!("nochg {}",&ip), "The supplied IP address is already set for this host. You should not attempt another update until your IP address changes.".into());
            
            Arc::new(value)
        }
        Err(err) => {
            error!("Failed to parse network responses.");
            return Err(Error::new(ErrorKind::Other,err.to_string()));
        }
    };

    let dns_cache = match read_dns_cache().await {
        Ok(value) => {
            Arc::new(Mutex::new(value))
        },
        Err(err) => {
            error!("Failed to load DNS Cache File | {}", err);
            return Err(Error::new(ErrorKind::Other,err));
        }
    };

    let dns_locked = match read_locked_dns().await {
        Ok(value) => Arc::new(Mutex::new(value)),
        Err(err) => {
            error!("Failed to load DNS Locked File | {}", err);
            return Err(Error::new(ErrorKind::Other, err));
        }
    };

    let updates = stream::iter(
        config.domains.iter().map(|domain|{
            let external_ip = ip.clone();
            let cache = dns_cache.clone();
            let locked = dns_locked.clone();
            let status_msgs = status.clone();
           
            async move {
                
                match locked.lock() {
                    Ok(value) => {
                        if value.contains(&domain.domain) {
                            warn!("Domain {} was not updated due to being locked.",domain.domain);
                            return Err("Failed to update due to lock.".to_string());
                        }
                    }
                    Err(err) => {
                        error!("Failed to optain a lock | {}", err);
                        return Err(err.to_string())
                    }
                }


                let needs_update = match cache.lock() {
                    Ok(lock) => {
                        match lock.get(&domain.domain) {
                            Some(value) => value.ip != external_ip,
                            None => true                   
                        }
                    }
                    Err(err) => {
                        error!("Failed to optain a lock | {}", err);
                        return Err(err.to_string())
                    }
                };

                if !needs_update {
                    info!("Domain {} is up to date.", domain.domain);
                    return Ok(());
                }

                let update_url = format_request(domain,&external_ip);
                let builder = reqwest::ClientBuilder::new();
                let client = match builder.user_agent(APP_USER_AGENT).build() {
                    Ok(value) => value,
                    Err(err) => {
                        error!("{}",err.to_string());
                        return Err(err.to_string());
                    }
                };

                // See https://support.google.com/domains/answer/6147083?hl=en#zippy=%2Cuse-the-api-to-update-your-dynamic-dns-record
                match client.get(&update_url).send().await {
                    Ok(res) => {
                        let stat = res.status();
                        let text = match res.text().await {
                            Ok(value) => value,
                            Err(err) => return Err(err.to_string())
                        };

                        let parsed_response = match status_msgs.get(&text) {
                            Some(value) => value,
                            None => &text
                        };

                        // Something happend server side, most likely.
                        // stop system from writing it to a cache file
                        if text.contains("911") {
                            error!("Domain({}) | {}", domain.domain,parsed_response);
                            return Ok(());
                        }

                        if stat.is_success() {

                            info!("Domain ({}) | {}",domain.domain,parsed_response);

                            match cache.lock() {
                                Ok(mut lock) => {
                                    lock.insert(domain.domain.clone(), config::CachedDns::new(external_ip, Utc::now().to_rfc2822()));
                                }
                                Err(err) => {
                                    error!("Failed to optain a lock. | {}", err);
                                    return Err(err.to_string())
                                }
                            }

                        } else {

                             error!("Domain({}) | {}", domain.domain, parsed_response);

                             match locked.lock() {
                                Ok(mut value) => {
                                    value.push(domain.domain.clone());
                                }
                                Err(err) => {
                                    error!("Failed to optain a lock. | {}", err);
                                    return Err(err.to_string())
                                }
                            }
                        }
                    }
                    Err(err) => return Err(err.to_string())
                }


                sleep(Duration::from_secs(2)).await;
                Ok(())
            }
        })
    ).buffered(1).collect::<Vec<Result<(),String>>>();


    updates.await;

    match Arc::try_unwrap(dns_cache) {
        Ok(arc) => {
            match arc.into_inner() {
                Ok(value) => {
                    if let Err(err) = write_dns_cache(value).await {
                        error!("Failed to write DNS Cache file | {}",err);
                        return Err(Error::new(ErrorKind::Other,err));
                      }
                }
                Err(err) => {
                    error!("Failed to optain a lock | {}", err);
                }
            }
        }
        Err(_) => {
            error!("Failed to write DNS Cache File");
        }
    };

    match Arc::try_unwrap( dns_locked) {
        Ok(arc) => {
            match arc.into_inner() {
                Ok(value) => {
                    if let Err(err) = write_locked_dns(value).await {
                        error!("Failed to write Locked DNS file | {}", err);
                        return Err(Error::new(ErrorKind::Other,err));
                    }
                }
                Err(err) => {
                    error!("Failed to optain a lock | {}", err);
                }
            }
        }
        Err(_) => {
            error!("Failed to write DNS Lock file");
        }
    };

    Ok(())
}