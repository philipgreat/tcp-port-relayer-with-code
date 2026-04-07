pub const USAGE: &str = "用法: ./tcp-auth-proxy --http-port=<http_port> --auth-key=<auth_key> <listen_port>-<dest> [<listen_port>-<dest> ...]\n";

pub struct AppConfig {
    pub http_port: u16,
    pub auth_key: String,
    pub proxy_rules: Vec<ProxyRule>,
}

pub struct ProxyRule {
    pub listen_port: u16,
    pub dest_addr: String,
}

pub fn parse_config(args: &[String]) -> Result<AppConfig, String> {
    let mut http_port = None;
    let mut auth_key = None;
    let mut proxy_rules = Vec::new();

    for arg in args {
        if let Some(value) = arg.strip_prefix("--http-port=") {
            let port = value
                .parse()
                .map_err(|_| format!("http_port 无法解析: `{}`", value))?;
            http_port = Some(port);
            continue;
        }

        if let Some(value) = arg.strip_prefix("--auth-key=") {
            auth_key = Some(value.to_string());
            continue;
        }

        proxy_rules.push(parse_proxy_rule(arg)?);
    }

    let http_port = http_port.ok_or_else(|| "缺少参数: --http-port=<http_port>".to_string())?;
    let auth_key = auth_key.ok_or_else(|| "缺少参数: --auth-key=<auth_key>".to_string())?;

    if proxy_rules.is_empty() {
        return Err("至少需要一组 <listen_port>-<dest>".to_string());
    }

    Ok(AppConfig {
        http_port,
        auth_key,
        proxy_rules,
    })
}

fn parse_proxy_rule(raw_arg: &str) -> Result<ProxyRule, String> {
    let parts: Vec<&str> = raw_arg.split('-').collect();
    if parts.len() < 2 {
        return Err(format!("`{}` 不是 <listen_port>-<dest>", raw_arg));
    }

    let listen_port = parts[0]
        .parse()
        .map_err(|_| format!("listen_port 无法解析: `{}`", parts[0]))?;

    let dest_addr = if parts[1].contains(':') {
        parts[1].to_string()
    } else {
        let port: u16 = parts[1]
            .parse()
            .map_err(|_| format!("dest 无法解析: `{}`", parts[1]))?;
        format!("127.0.0.1:{}", port)
    };

    Ok(ProxyRule {
        listen_port,
        dest_addr,
    })
}
