// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use std::net::IpAddr;

#[derive(PartialEq, Eq, Debug)]
pub enum Proto {
    HTTP,
    HTTPS,
    COAP,
}

/// Parse the host and port from an url
///
/// From the project references, it is mainly used to bind socket and start listening. As
/// such, from usage, it must return an IP because the socket API will not resolve names.
///
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
                    return Err(anyhow::anyhow!("Protocol Parse Error, got '{:?}'", raw));
                }
            };
            let port = match val[3].parse() {
                Ok(prt) => prt,
                Err(_) => {
                    return Err(anyhow::anyhow!("Port Parse Error, got '{:?}'", raw));
                }
            };
            let maybe_an_ip = val[2]
                .parse::<IpAddr>() // Try to parse an IP
                .ok() // convert to Option
                // If None (i.e no IP), remove the eventual bracket prefix and suffix (for IPv6) then try again
                .or_else(|| val[2].strip_prefix("[")?.strip_suffix("]")?.parse::<IpAddr>().ok());
            let host = match maybe_an_ip {
                Some(ip) => ip.to_string(), // WARNING the verbatim IP in `raw` may be outputed in a canonical form
                None => {
                    // FIXME: the val[2] could still be an hostname or dns name
                    //    The next step should be to try to resolve it to an IP
                    //    or return an error:  `return Err(anyhow::anyhow!("Host Parse Error"))`
                    //    But to keep the previous API behavior, we return the host verbatim
                    // For example with:
                    // let resolve = tokio::runtime::Runtime::new()
                    //     .unwrap()
                    //     .block_on(tokio::net::lookup_host(val[2].to_string()));
                    // resolve?.last().map_or(val[2].to_string(), |v| v.to_string())
                    let fallback = val[2].trim().to_string();
                    if fallback.is_empty() {
                        return Err(anyhow::anyhow!("Host Parse Error, got '{:?}'", raw));
                    }
                    fallback
                }
            };
            Ok((proto, host, port))
        }
        None => {
            return Err(anyhow::anyhow!("Regexp Parse Error, got '{:?}'", raw));
        }
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

#[cfg(feature = "grpc_impl")]
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[cfg(feature = "grpc_impl")]
    #[test]
    /// Assert the function behavior versus possible inputs
    fn test_parse_http_host() {
        let result = parse_http_host("http://192.168.3.3");
        assert!(result.is_err(), "Missing port");
        let result = parse_http_host("http://127.0.0.1:7035");
        assert_eq!(result.unwrap(), (Proto::HTTP, String::from("127.0.0.1"), 7035u16));
        let result = parse_http_host("http://example.com:7035");
        assert_eq!(result.unwrap(), (Proto::HTTP, String::from("example.com"), 7035u16));
        let result: Result<(Proto, String, u16), anyhow::Error> = parse_http_host("http://[::1]:7035");
        assert_eq!(result.unwrap(), (Proto::HTTP, String::from("::1"), 7035u16));
        let result = parse_http_host("http://[2a01:4f8:212:fa01::4]:7035");
        assert_eq!(result.unwrap(), (Proto::HTTP, String::from("2a01:4f8:212:fa01::4"), 7035u16));
        let result: Result<(Proto, String, u16), anyhow::Error> = parse_http_host("http://[0:0:0:0:0:FFFF:129.144.52.38]:7035");
        assert!(result.is_ok());
        let (_, result, _) = result.unwrap();
        assert!(matches!(result.as_str(), "0:0:0:0:0:FFFF:129.144.52.38" | "::ffff:129.144.52.38"));
        let result = parse_http_host("http://[::13.1.68.3]:7035");
        assert!(result.is_ok());
        let (_, result, _) = result.unwrap();
        assert!(matches!(result.as_str(), "::13.1.68.3" | "::d01:4403"));
    }

    #[cfg(feature = "grpc_impl")]
    #[test]
    /// Check that the standard library for socket only works with IPs.
    fn check_socket_family() {
        use std::net::SocketAddr;

        let addr = SocketAddr::from((IpAddr::from_str("127.0.0.1").unwrap(), 7035u16));
        assert!(addr.is_ipv4());
        let addr = SocketAddr::from((IpAddr::from_str("::1").unwrap(), 7035u16));
        assert!(addr.is_ipv6());
        let addr = SocketAddr::from((IpAddr::from_str("::13.1.68.3").unwrap(), 7035u16));
        assert!(addr.is_ipv6());
        assert!(IpAddr::from_str("localhost").is_err())
    }

    #[cfg(feature = "grpc_impl")]
    #[test]
    /// Check that parsing IP works with any IPs and not with hostname.
    fn check_host_is_parsed() {
        use std::net::{Ipv4Addr, Ipv6Addr, ToSocketAddrs};

        let result = IpAddr::from_str("127.0.0.1");
        assert_eq!(result.unwrap(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        let result = IpAddr::from_str("::13.1.68.3");
        assert_eq!(result.unwrap(), IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0x0d01, 0x4403)));
        let result = IpAddr::from_str("localhost");
        assert!(result.is_err(), "std::net::IpAddr::from_str cannot resolve hostname or dns name");

        let result = "localhost:7035".to_socket_addrs();
        assert!(result.is_ok());
        assert!(result.unwrap().last().map_or(false, |v| v.ip().is_loopback()));
    }
}
