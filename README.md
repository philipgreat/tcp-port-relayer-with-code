# tcp-port-relayer-with-code

## 用法

程序支持一次传入多组参数，每组参数格式如下：

```bash
./tcp-auth-proxy --http-port=<http_port> --auth-key=<auth_key> [--mock-ip=<ip>] [--run-as-client=true|false] [--run-on-host=<servername>] [<listen_port>-<dest> ...]
```

示例：

```bash
./tcp-auth-proxy --http-port=8080 --auth-key=123123sdfasdf 9001-127.0.0.1:22 9002-3306-mysql
```

启用 hash 授权示例：

```bash
./tcp-auth-proxy --http-port=8080 --auth-key=123123sdfasdf 9001-127.0.0.1:22
```

生成指定客户端 IP 的授权 URL 并立即退出：

```bash
./tcp-auth-proxy --http-port=8080 --auth-key=123123sdfasdf --mock-ip=203.0.113.10
```

作为客户端自动获取本机出口 IP 并完成授权：

```bash
./tcp-auth-proxy --http-port=8080 --auth-key=123123sdfasdf --run-as-client=true --run-on-host=relay.example.com
```

指定对外访问地址后打印完整 URL：

```bash
./tcp-auth-proxy --http-port=8080 --auth-key=123123sdfasdf --mock-ip=203.0.113.10 --run-on-host=relay.example.com
```

如果 `servername` 自带端口，则直接使用：

```bash
./tcp-auth-proxy --http-port=8080 --auth-key=123123sdfasdf --run-on-host=relay.example.com:9000 9001-127.0.0.1:22
```

说明：

- `--http-port` 和 `--auth-key` 是全局参数，只需要传一次
- 授权 URL 中的 key 固定使用 `sha256(client_ip + auth_key)` 的小写 hex
- `--mock-ip=<ip>` 表示不启动服务，直接打印该客户端 IP 对应的授权 URL，然后进程退出
- 传了 `--mock-ip=<ip>` 时，不需要再提供任何 `<listen_port>-<dest>`
- `--run-as-client=true` 表示作为客户端运行：请求 `http://<host>:<http-port>/ip` 取得出口 IP，计算 `sha256(ip + auth_key)` 的小写 hex，再请求对应授权 URL 并打印 HTTP 返回内容
- `--run-as-client=true` 时必须提供 `--run-on-host=<servername>`
- `--run-on-host=<servername>` 用于生成和展示完整管理地址；如果不带端口，则自动拼上 `--http-port`
- 后面的每组转发规则只解析前两段：`listen_port`、`dest`
- 第二段后面的内容会被忽略，可以作为备注使用
- 例如 `9002-3306-mysql` 会按 `dest=3306` 处理
- `dest` 如果是纯端口号，会自动转成 `127.0.0.1:<port>`
- `dest` 中不支持包含 `-`
- 同一次启动里的所有转发端口共用同一个授权状态，授权一次 IP 后可访问全部转发规则

管理接口：

- `GET /ip` 返回当前请求方的 IP 地址
- `GET /list` 返回当前已授权 IP 列表
- `GET /<key>` 校验 key 成功后，将当前请求方 IP 加入授权列表

客户端脚本：

- 可使用 [scripts/auth-client.sh](/Users/Philip/githome/tcp-port-relayer-with-code/scripts/auth-client.sh) 自动从远端 `/ip` 获取当前出口 IP，计算授权 URL，并发起授权请求

```bash
./scripts/auth-client.sh http://relay.example.com:8080 987654321
```

## 构建 armv7 Linux 静态版本

依赖：

- `zig`
- `cargo-zigbuild`
- Rust target: `armv7-unknown-linux-musleabihf`

首次构建先安装 target：

```bash
rustup target add armv7-unknown-linux-musleabihf
```

编译命令：

```bash
cargo zigbuild --release --target armv7-unknown-linux-musleabihf
```

或者直接执行：

```bash
make armv7-musl
```

输出文件：

```bash
target/armv7-unknown-linux-musleabihf/release/tcp-auth-proxy
```

说明：

- 这是静态 musl 版本，部署时不依赖目标机器上的 glibc
- 产物没有 interpreter，这是静态链接的正常表现
