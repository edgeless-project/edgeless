pub enum HttpProto {
    HTTP,
    HTTPS,
}
pub fn parse_http_host(raw: &str) -> anyhow::Result<(HttpProto, String, u16)> {
    let re = regex::Regex::new(r"(http[s]?):\/\/(.*):(\d+)").unwrap();
    let res = re.captures(raw);
    match res {
        Some(val) => {
            let proto = if &val[1] == "http" { HttpProto::HTTP } else { HttpProto::HTTPS };
            let port = match val[3].parse() {
                Ok(prt) => prt,
                Err(_) => {
                    return Err(anyhow::anyhow!("Host Parse Error"));
                }
            };
            Ok((proto, val[2].to_string(), port))
        }
        None => {
            return Err(anyhow::anyhow!("Host Parse Error"));
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
