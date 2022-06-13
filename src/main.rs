mod config;
mod network;

use config::read_locked_dns;
use config::write_dns_cache;
use config::write_locked_dns;
use network::{get_exteral_ip, format_request};
use config::{load_config, read_dns_cache};
use futures::StreamExt;
use futures::stream;
use std::collections::HashMap;
use std::io::ErrorKind;
use std::io::Error;
use std::sync::{ Arc, Mutex };
use chrono::Utc;
use casual_logger::{Log, Extension, Opt};

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"),"/",env!("CARGO_PKG_VERSION"),);


#[tokio::main]
async fn main() -> std::io::Result<()> {
    Log::remove_old_logs();
    Log::set_file_ext(Extension::Log);
    Log::set_file_name("");
    Log::set_opt(Opt::Release);
    
    let config = match load_config().await {
        Ok(value) => value,
        Err(err) => {
            Log::fatal(&format!("Failed to load config. | {}",err));
            return Err(Error::new(ErrorKind::Other,err))
        }
    };
    
    let ip = match get_exteral_ip().await {
        Ok(value) => value,
        Err(err) => {
            Log::fatal(&format!("Failed to get external ip | {}",err));
            return Err(Error::new(ErrorKind::Other,err));
        }
    };
    Log::info(&format!("Current external IP: {}",&ip));

    let status_file = include_str!("request.json");
    let status = match serde_json::from_str::<HashMap<String,String>>(status_file) {
        Ok(mut value) => {

            value.insert(format!("good {}",&ip), "The update was successful. You should not attempt another update until your IP address changes.".into());
            value.insert(format!("nochg {}",&ip), "The supplied IP address is already set for this host. You should not attempt another update until your IP address changes.".into());
            
            Arc::new(value)
        }
        Err(err) => {
            Log::fatal("Failed to parse request.json");
            return Err(Error::new(ErrorKind::Other,err.to_string()));
        }
    };

    let dns_cache = match read_dns_cache().await {
        Ok(value) => {
            Arc::new(Mutex::new(value))
        },
        Err(err) => {
            Log::fatal(&format!("Failed to read DNS Cache | {}",err));
            return Err(Error::new(ErrorKind::Other,err));
        }
    };

    let dns_locked = match read_locked_dns().await {
        Ok(value) => Arc::new(Mutex::new(value)),
        Err(err) => {
            Log::fatal(&err);
            return Err(Error::new(ErrorKind::Other, err));
        }
    };


    let updates = stream::iter(
        config.domains.iter().map(|domain|{
            let external_ip = ip.clone();
            let arc = dns_cache.clone();
            let status_msg = status.clone();
            let errored = dns_locked.clone();
            async move {
                
                match errored.lock() {
                    Ok(value) => {
                        if value.contains(&domain.domain) {
                            let msg = format!("Domain {} had an error and is locked",&domain.domain);
                            Log::error(&msg);
                            return Err(msg);
                        }
                    }
                    Err(err) => {
                        Log::error("Failed to lock");
                        return Err(err.to_string())
                    }
                }


                let needs_update = match arc.lock() {
                    Ok(lock) => {
                        match lock.get(&domain.domain) {
                            Some(value) => value.ip != external_ip,
                            None => true                   
                        }
                    }
                    Err(err) => {
                        Log::error("Failed to lock");
                        return Err(err.to_string())
                    }
                };

                if !needs_update {
                    Log::info(&format!("{} is up to date",&domain.domain));
                    return Ok(());
                }

                let update_url = format_request(domain);
                let builder = reqwest::ClientBuilder::new();
                let client = match builder.user_agent(APP_USER_AGENT).build() {
                    Ok(value) => value,
                    Err(err) => {
                        Log::error(&err.to_string());
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

                        let msg = match status_msg.get(&text) {
                            Some(value) => value,
                            None => &text
                        };
                        
                        if stat.is_success() {
                            Log::info(msg);

                            match arc.lock() {
                                Ok(mut lock) => {
                                    lock.insert(domain.domain.clone(), config::CachedDns::new(external_ip, Utc::now().to_rfc2822()));
                                }
                                Err(err) => {
                                    Log::error("Failed to lock");
                                    return Err(err.to_string())
                                }
                            }
                        } else {

                            if !msg.contains("911") {
                                match errored.lock() {
                                    Ok(mut value) => {
                                        value.push(domain.domain.clone());
                                    }
                                    Err(err) => {
                                        Log::error("Failed to lock");
                                        return Err(err.to_string())
                                    }
                                }
                            }
                            
                            Log::error(&format!("{} | {}",msg,domain.domain));
                        }
                    }
                    Err(err) => return Err(err.to_string())
                }


                Ok(())
            }
        })
    ).buffer_unordered(8).collect::<Vec<Result<(),String>>>();


    updates.await;

    match Arc::try_unwrap(dns_cache) {
        Ok(arc) => {
            match arc.into_inner() {
                Ok(value) => {
                    if let Err(err) = write_dns_cache(value).await {
                        Log::fatal(&err);
                        return Err(Error::new(ErrorKind::Other,err));
                      }
                }
                Err(err) => {
                    Log::error(&err.to_string());
                }
            }
        }
        Err(_) => {
            Log::error("DNS Cache write failed. Failed to unwrap arc");
        }
    };

    match Arc::try_unwrap( dns_locked) {
        Ok(arc) => {
            match arc.into_inner() {
                Ok(value) => {
                    if let Err(err) = write_locked_dns(value).await {
                        Log::fatal(&err);
                        return Err(Error::new(ErrorKind::Other,err));
                    }
                }
                Err(err) => {
                    Log::error(&err.to_string());
                }
            }
        }
        Err(_) => {
            Log::error("DNS Cache write failed. Failed to unwrap arc");
        }
    };

    Log::flush();
    Ok(())
}