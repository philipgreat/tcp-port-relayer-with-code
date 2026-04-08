pub const USAGE: &str = "用法: ./tcp-auth-proxy --http-port=<http_port> [--auth-key=<auth_key>] [--run-as-client=true|false] --run-on-host=<servername> [<listen_port>-<dest> ...]\n";

pub struct AppConfig {
    pub http_port: u16,
    pub auth_key: Option<String>,
    pub run_as_client: bool,
    pub run_on_host: Option<String>,
    pub proxy_rules: Vec<ProxyRule>,
}

pub struct ProxyRule {
    pub listen_port: u16,
    pub dest_addr: String,
}

pub fn parse_config(args: &[String]) -> Result<AppConfig, String> {
    let mut http_port = None;
    let mut auth_key = None;
    let mut run_as_client = false;
    let mut run_on_host = None;
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

        if let Some(value) = arg.strip_prefix("--run-as-client=") {
            run_as_client = match value {
                "true" => true,
                "false" => false,
                _ => {
                    return Err(format!(
                        "--run-as-client 只能是 true 或 false: `{}`",
                        value
                    ))
                }
            };
            continue;
        }

        if let Some(value) = arg.strip_prefix("--run-on-host=") {
            if value.is_empty() {
                return Err("--run-on-host 不能为空".to_string());
            }
            run_on_host = Some(value.to_string());
            continue;
        }

        proxy_rules.push(parse_proxy_rule(arg)?);
    }

    let http_port = http_port.ok_or_else(|| "缺少参数: --http-port=<http_port>".to_string())?;
    if run_as_client && auth_key.is_none() {
        return Err("缺少参数: --auth-key=<auth_key>".to_string());
    }

    if run_on_host.is_none() {
        return Err("缺少参数: --run-on-host=<servername>".to_string());
    }

    if proxy_rules.is_empty() && !run_as_client {
        return Err("至少需要一组 <listen_port>-<dest>".to_string());
    }

    Ok(AppConfig {
        http_port,
        auth_key,
        run_as_client,
        run_on_host,
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

#[cfg(test)]
mod tests {
    use super::parse_config;

    #[test]
    fn run_as_client_requires_run_on_host() {
        let args = vec![
            "--http-port=8080".to_string(),
            "--auth-key=987654321".to_string(),
            "--run-as-client=true".to_string(),
        ];

        match parse_config(&args) {
            Ok(_) => panic!("expected parse_config to fail without --run-on-host"),
            Err(err) => assert_eq!(err, "缺少参数: --run-on-host=<servername>"),
        }
    }

    #[test]
    fn run_as_client_allows_missing_proxy_rules() {
        let args = vec![
            "--http-port=8080".to_string(),
            "--auth-key=987654321".to_string(),
            "--run-as-client=true".to_string(),
            "--run-on-host=relay.example.com".to_string(),
        ];

        let config = parse_config(&args).unwrap();
        assert!(config.run_as_client);
        assert!(config.proxy_rules.is_empty());
        assert_eq!(config.run_on_host.as_deref(), Some("relay.example.com"));
    }

    #[test]
    fn run_as_client_requires_auth_key() {
        let args = vec![
            "--http-port=8080".to_string(),
            "--run-as-client=true".to_string(),
            "--run-on-host=relay.example.com".to_string(),
        ];

        match parse_config(&args) {
            Ok(_) => panic!("expected parse_config to fail without --auth-key"),
            Err(err) => assert_eq!(err, "缺少参数: --auth-key=<auth_key>"),
        }
    }

    #[test]
    fn server_mode_allows_missing_auth_key() {
        let args = vec![
            "--http-port=8080".to_string(),
            "--run-on-host=relay.example.com".to_string(),
            "9001-127.0.0.1:22".to_string(),
        ];

        let config = parse_config(&args).unwrap();
        assert!(config.auth_key.is_none());
        assert_eq!(config.proxy_rules.len(), 1);
    }

    #[test]
    fn server_mode_requires_run_on_host() {
        let args = vec![
            "--http-port=8080".to_string(),
            "9001-127.0.0.1:22".to_string(),
        ];

        match parse_config(&args) {
            Ok(_) => panic!("expected parse_config to fail without --run-on-host"),
            Err(err) => assert_eq!(err, "缺少参数: --run-on-host=<servername>"),
        }
    }
}
