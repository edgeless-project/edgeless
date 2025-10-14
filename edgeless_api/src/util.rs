// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
#[derive(PartialEq, Eq)]
pub enum Proto {
    HTTP,
    HTTPS,
    COAP,
}

impl std::fmt::Display for Proto {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::HTTP => "http",
                Self::HTTPS => "https",
                Self::COAP => "coap",
            }
        )
    }
}

pub fn parse_http_host(raw: &str) -> anyhow::Result<(Proto, String, u16)> {
    let re = regex::Regex::new(r"(\w+):\/\/(.*):(\d+)").unwrap();
    let res = re.captures(raw);
    match res {
        Some(val) => {
            let proto = match &val[1] {
                "http" => Proto::HTTP,
                "https" => Proto::HTTPS,
                "coap" => Proto::COAP,
                _ => {
                    return Err(anyhow::anyhow!("Host Parse Error"));
                }
            };
            let port = match val[3].parse() {
                Ok(prt) => prt,
                Err(_) => {
                    return Err(anyhow::anyhow!("Host Parse Error"));
                }
            };
            Ok((proto, val[2].to_string(), port))
        }
        None => Err(anyhow::anyhow!("Host Parse Error")),
    }
}

pub fn create_template(path: &str, content: &str) -> anyhow::Result<()> {
    assert!(!path.is_empty());
    match std::path::Path::new(&path).exists() {
        true => anyhow::bail!("cannot overwrite configuration file: {}", path),
        false => {
            std::fs::write(path, content)?;
            Ok(())
        }
    }
}

pub fn get_my_ip() -> anyhow::Result<String> {
    let interfaces = get_if_addrs::get_if_addrs()?;
    for iface in interfaces {
        if iface.is_loopback() {
            continue;
        }
        match iface.ip() {
            std::net::IpAddr::V4(ip) => {
                return Ok(ip.to_string());
            }
            std::net::IpAddr::V6(_) => {
                continue;
            }
        }
    }
    anyhow::bail!("cannot find a suitable IP address");
}

pub fn get_announced(url: &str, announced_url: &str) -> anyhow::Result<String> {
    if !announced_url.is_empty() {
        let _ = parse_http_host(announced_url)?;
        Ok(announced_url.to_string())
    } else {
        let (proto, _address, port) = parse_http_host(url)?;
        let my_ip = get_my_ip()?;
        Ok(format!("{}://{}:{}", proto, my_ip, port))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_my_ip() {
        let ip = get_my_ip().unwrap();
        println!("IP: {}", ip);
        assert!(!ip.is_empty());
    }

    #[test]
    fn test_get_announced() {
        let url = "http://10.0.0.1:1234";

        let announced = get_announced(url, "");
        assert!(announced.is_ok());
        assert_eq!(
            format!("http://{}:1234", get_my_ip().unwrap()),
            announced.unwrap()
        );

        let announced = get_announced(url, "http://1.2.3.4:1234");
        assert!(announced.is_ok());
        assert_eq!(String::from("http://1.2.3.4:1234"), announced.unwrap());

        let announced = get_announced("invalid-url", "");
        assert!(announced.is_err());

        let announced = get_announced(url, "invalid-url");
        assert!(announced.is_err());
    }
}
