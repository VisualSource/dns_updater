extern crate reqwest;
extern crate chrono;
#[macro_use] extern crate json;

use chrono::{Utc};
use json::JsonValue;
use std::fs::read_to_string;
use std::fs::write;
use std::collections::hash_map;

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
    let mut request_status = hash_map::HashMap::<String,String>::new();
    request_status.insert("nohost".to_string(),"The hostname doesn't exist, or doesn't have Dynamic DNS enabled.".to_string());
    request_status.insert("badauth".to_string(),"The username/password combination isn't valid for the specified host.".to_string());
    request_status.insert("notfqdn".to_string(),"The supplied hostname isn't a valid fully-qualified domain name.".to_string());
    request_status.insert("badagent".to_string(),"Your Dynamic DNS client makes bad requests. Ensure the user agent is set in the request.".to_string());
    request_status.insert("abuse".to_string(),"Dynamic DNS access for the hostname has been blocked due to failure to interpret previous responses correctly.".to_string());
    request_status.insert("911".to_string(),"An error happened on our end. Wait 5 minutes and retry.".to_string());
    request_status.insert("conflict A".to_string(),"A custom A or AAAA resource record conflicts with the update. Delete the indicated resource record within the DNS settings page and try the update again.".to_string());
    request_status.insert("conflict AAAA".to_string(),"A custom A or AAAA resource record conflicts with the update. Delete the indicated resource record within the DNS settings page and try the update again.".to_string());

    let config = load_config().unwrap();
    let is_debug = config["debug"].as_bool().unwrap();

    let mut dns = read_current_dns(is_debug).unwrap();
   
    let ip = get_exteral_ip(is_debug).unwrap();

    request_status.insert(format!("good {}",ip.clone()).to_string(), String::from("The update was successful. You should not attempt another update until your IP address changes."));
    request_status.insert(format!("nochg {}",ip.clone()).to_string(), String::from("The supplied IP address is already set for this host. You should not attempt another update until your IP address changes."));


    for key in config["domains"].members() {
        let domain_name = key["domain"].to_string();
        let domain = &dns[domain_name.clone()];
        if domain["ip"] != ip {
            dns[domain_name.clone()] = object!{
                ip: ip.clone(),
                changed: Utc::now().to_rfc2822()
            };
            if is_debug { println!("IP of {} does not match exterial",domain_name.clone()); }
        }
        let request = create_request(&key ,&ip, is_debug);
        if !is_debug { 
            match reqwest::blocking::get(request) {
                Ok(value) => {
                    match value.text() {
                        Ok(result) => {
                            println!("{}",request_status[&result]);
                        }
                        Err(parse_err) => {
                            eprintln!("HTTP UPDATE REQUEST ERROR: {}",parse_err);
                        }
                    }
                }
                Err(err) => {
                    eprintln!("HTTP UPDATE REQUEST ERROR: {}",err);
                }
            }
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