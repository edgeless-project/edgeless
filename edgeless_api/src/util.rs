// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
#[derive(PartialEq, Eq)]
pub enum Proto {
    HTTP,
    HTTPS,
    COAP,
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
