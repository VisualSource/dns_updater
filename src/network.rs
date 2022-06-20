use crate::config::Domain;


pub fn format_request(domain: &Domain) -> String {
    // og https://domains.google.com
    format!("https://{username}:{psd}@domains.google.com/nic/update?hostname={domain}",
    domain=domain.domain,
    psd=domain.psd,
    username=domain.username
    )
}

pub async fn get_exteral_ip() -> Result<String, String>   {
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
        match get_exteral_ip().await {
            Ok(value)=> println!("{}",value),
            Err(err) => eprintln!("{}",err)
        }
    }

    #[test]
    fn test_format_request(){
        let domain = Domain { username: "USERNAME".into(), psd: "PSD".into(), domain: "DOMAIN".into() };

        let request = format_request(&domain);

        assert_eq!(request,"https://USERNAME:PSD@domains.google.com/nic/update?hostname=DOMAIN".to_string())

    }

}