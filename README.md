# tcp-port-relayer-with-code

## Usage

The program accepts one or more forwarding rules in a single invocation:

```bash
./tcp-auth-proxy --http-port=<http_port> [--auth-key=<auth_key>] [--run-as-client=true|false] --run-on-host=<servername> [<listen_port>-<dest> ...]
```

Example:

```bash
./tcp-auth-proxy --http-port=8080 --auth-key=123123sdfasdf --run-on-host=relay.example.com 9001-127.0.0.1:22 9002-3306-mysql
```

Server mode with an auto-generated auth key:

```bash
./tcp-auth-proxy --http-port=8080 --run-on-host=relay.example.com 9001-127.0.0.1:22
```

Client mode that fetches the current public IP and authorizes it:

```bash
./tcp-auth-proxy --http-port=8080 --auth-key=123123sdfasdf --run-as-client=true --run-on-host=relay.example.com
```

If `servername` already includes a port, it is used as-is:

```bash
./tcp-auth-proxy --http-port=8080 --auth-key=123123sdfasdf --run-on-host=relay.example.com:9000 9001-127.0.0.1:22
```

Notes:

- `--http-port` and `--auth-key` are global parameters and only need to be provided once
- In server mode, `--auth-key` is optional. If omitted, a strong random key is generated and printed at startup
- Authorization keys always use lowercase hex `sha256(client_ip + auth_key)`
- `--run-as-client=true` runs in client mode: it requests `http://<host>:<http-port>/ip`, computes lowercase hex `sha256(ip + auth_key)`, then requests the authorization URL and prints the HTTP response
- `--run-on-host=<servername>` is required in both server mode and client mode. If it does not include a port, `--http-port` is appended automatically
- `--run-as-client=true` also requires `--auth-key=<auth_key>`
- Only the first two segments of each forwarding rule are parsed: `listen_port`, `dest`
- Any content after the second segment is ignored and can be used as a note
- For example, `9002-3306-mysql` is treated as `dest=3306`
- If `dest` is just a port number, it is converted to `127.0.0.1:<port>`
- `dest` cannot contain `-`
- All forwarding ports in the same process share one authorization state; authorizing one IP unlocks every forwarding rule in that process

Management endpoints:

- `GET /` loads `auth.html` from the current working directory at runtime and returns the auth page; if the file is missing, the request returns an error
- `GET /ip` returns the requester's IP address
- `GET /list` returns the list of currently authorized IPs
- `GET /<key>` validates the key and adds the requester's IP to the authorized list

## Build Armv7 Linux Static Binary

Requirements:

- `zig`
- `cargo-zigbuild`
- Rust target: `armv7-unknown-linux-musleabihf`

Install the Rust target first:

```bash
rustup target add armv7-unknown-linux-musleabihf
```

Build command:

```bash
cargo zigbuild --release --target armv7-unknown-linux-musleabihf
```

Or run:

```bash
make armv7-musl
```

Output file:

```bash
target/armv7-unknown-linux-musleabihf/release/tcp-auth-proxy
```

Notes:

- This is a static musl build and does not depend on glibc on the target machine
- The output binary has no interpreter, which is expected for a static binary
