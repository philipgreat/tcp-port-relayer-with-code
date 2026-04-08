mod auth;
mod config;
mod proxy;

use crate::config::{parse_config, USAGE};
use crate::proxy::start_proxy;
use std::env;
use std::future::pending;

#[tokio::main]
async fn main() {
    println!("============= BUILD at {}  by {}@{} ====================\n", 
        env!("BUILD_TIME"),
        env!("BUILD_USER"),
        env!("BUILD_HOSTNAME"));
    

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("{}", USAGE);
        return;
    }

    let config = match parse_config(&args[1..]) {
        Ok(config) => config,
        Err(err) => {
            println!("参数格式错误: {}", err);
            println!("{}", USAGE);
            return;
        }
    };

    let should_exit = config.run_as_client;

    if let Err(err) = start_proxy(config).await {
        println!("启动失败: {}", err);
        return;
    }

    if should_exit {
        return;
    }

    pending::<()>().await;
}
