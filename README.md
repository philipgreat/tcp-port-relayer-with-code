# tcp-port-relayer-with-code

## 用法

程序支持一次传入多组参数，每组参数格式如下：

```bash
./tcp-auth-proxy --http-port=<http_port> --auth-key=<auth_key> <listen_port>-<dest> [<listen_port>-<dest> ...]
```

示例：

```bash
./tcp-auth-proxy --http-port=8080 --auth-key=123123sdfasdf 9001-127.0.0.1:22 9002-3306-mysql
```

说明：

- `--http-port` 和 `--auth-key` 是全局参数，只需要传一次
- 后面的每组转发规则只解析前两段：`listen_port`、`dest`
- 第二段后面的内容会被忽略，可以作为备注使用
- 例如 `9002-3306-mysql` 会按 `dest=3306` 处理
- `dest` 如果是纯端口号，会自动转成 `127.0.0.1:<port>`
- `dest` 中不支持包含 `-`
- 同一次启动里的所有转发端口共用同一个授权状态，授权一次 IP 后可访问全部转发规则

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
