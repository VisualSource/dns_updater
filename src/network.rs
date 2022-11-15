use crate::config::{Domain, Config};


pub fn format_request(domain: &Domain, ip: &String) -> String {

    // see https://domains.google.com
    format!("https://{username}:{psd}@domains.google.com/nic/update?hostname={domain}&myip={myip}",
    domain=domain.domain,
    psd=domain.psd,
    username=domain.usr,
    myip=ip
    )
}




pub async fn get_exteral_ip(config: &Config) -> Result<String, String>   {

    if config.debug {
        return Ok(config.debug_ip.clone().unwrap_or_default());
    }

    match reqwest::get("https://domains.google.com/checkip").await {
        Ok(response) => {
            match response.text().await {
                Ok(text) => {
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


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_exteral_ip(){
        let mut config = Config::default();

        match get_exteral_ip(&config).await {
            Ok(value)=> println!("{}",value),
            Err(err) => panic!("Failed to get Exteral IP | {}",err)
        }

        // test debug mode

        config.debug = true;

        match get_exteral_ip(&config).await {
            Ok(value) => assert_eq!(value,"".to_string()),
            Err(err) => panic!("Failed to get Exteral IP | {}", err)
        }
    }

    #[test]
    fn test_format_request(){
        let domain = Domain { usr: "USERNAME".into(), psd: "PSD".into(), domain: "DOMAIN".into() };

        let request = format_request(&domain,&"".to_string());

        assert_eq!(request,"https://USERNAME:PSD@domains.google.com/nic/update?hostname=DOMAIN&myip=".to_string())

    }

}