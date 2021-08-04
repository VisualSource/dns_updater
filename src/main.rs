extern crate reqwest;
extern crate chrono;
#[macro_use] extern crate json;

use chrono::{Utc};
use json::JsonValue;
use std::fs::read_to_string;
use std::fs::write;

fn load_config() -> Result<JsonValue,String> {
    match read_to_string("./config.json") {
        Ok(config) => {
           match json::parse(config.as_str()) {
               Ok(text) => {
                   Ok(text)
               }
               Err(parse_err) => {
                   eprintln!("{}",parse_err);
                   Err(parse_err.to_string())
               }
           }
        }
        Err(err) => {
            write("./config.json", b"{ \"domains\":[], \"debug\": false }").expect("Failed to create config file");
            eprintln!("{} | Creating new config file.",err);
            Ok(object!{
                domains: [],
                debug: false
            })
            
        }
    }
}

fn get_exteral_ip(debug: bool) -> Result<String, String>   {
    match reqwest::blocking::get("http://ifconfig.me/ip") {
        Ok(response) => {
            match response.text() {
                Ok(text) => {
                    if debug {
                        println!("Exteral IP: {}",text)
                    }
                    Ok(text)
                }
                Err(parse_err) => {
                    Err(parse_err.to_string())
                }
            }
        }
        Err(request_err) => {
            Err(request_err.to_string())
        }
    }
}

fn create_request(data: &json::JsonValue, ip: &String, debug: bool) -> String {
    let request = format!("https://{username}:{psd}@domains.google.com/nic/update?hostname={domain}&myip={ip}",
    ip=ip,
    domain=data["domain"].as_str().unwrap(),
    psd=data["psd"].as_str().unwrap(),
    username=data["username"].as_str().unwrap()
    ).to_string();
    if debug {
        println!("Request: {}",request);
    }
    request
}

fn read_current_dns(debug: bool) -> Result<JsonValue,String> {
    match read_to_string("./current_dns.json") {
        Ok(value) => {
            match json::parse(value.as_str()) {
                Ok(dns) => {
                    if debug {
                        println!("{:#?}",dns);
                    } 
                    Ok(dns)
                }
                Err(err)=> {
                    Err(err.to_string())
                }
            }
        }
        Err(_) => {
            match write("./current_dns.json", b"{}") {
                Ok(_) => {
                    if debug {
                        println!("Creating file, useing default");
                    } 
                    Ok(json::JsonValue::new_object())
                }
                Err(io_err) => {
                    Err(io_err.to_string())
                }
            }
        }
    }
}

fn main() -> Result<(), String> {
    let config = load_config().unwrap();
    let is_debug = config["debug"].as_bool().unwrap();

    let mut dns = read_current_dns(is_debug).unwrap();
   
    let ip = get_exteral_ip(is_debug).unwrap();

    for key in config["domains"].members() {
        let domain_name = key["domain"].to_string();
        let domain = &dns[domain_name.clone()];
        if domain["ip"] != ip {
            dns[domain_name] = object!{
                ip: ip.clone(),
                changed: Utc::now().to_rfc2822()
            };
            if is_debug { println!("IPs differ"); }
        }
        let request = create_request(&key ,&ip, is_debug);
        if !is_debug { 
            reqwest::blocking::get(request).unwrap(); 
        }
    }

    write("./current_dns.json", json::stringify_pretty(dns, 2)).expect("Failed to save current dns.");
      
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_current_dns_load(){
        match read_current_dns(true) {
            Ok(value) => {
                println!("{}",value);
            }
            Err(err) => {
                eprintln!("{}",err);
            }
        }
    }
    #[test]
    fn test_format_request(){
        match load_config() {
            Ok(value) => {
                println!("{:#?}",create_request(&value["domains"][0],&String::from("112.168.1.19"),true));
            }
            Err(err) => {
                eprintln!("ERROR: {}",err);
            }
        }
    }
    #[test]
    fn test_yaml_config_load(){
        match load_config() {
            Ok(value) => {
                println!("{:#?}",value["domains"]);
            }
            Err(err) => {
                eprintln!("ERROR: {}",err);
            }
        }
    }
    #[test]
    fn test_request_exteral_ip(){
        match get_exteral_ip(true) {
            Ok(value) => {
                println!("{}",value);
            }
            Err(err) => {
                println!("{}",err);
            }
        }
    }
}