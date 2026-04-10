all:
	cargo run -- --http-port=32001  --run-on-host=t420.doublechaintech.cn   32022-22-ssh 31001-192.168.1.245:5900-openclaw01 31002-192.168.1.127:5900-openclaw02
auth:
	cargo run -- --http-port=32001  --run-on-host=t420.doublechaintech.cn  --http-port=32001  --run-on-host=t420.doublechaintech.cn   --auth-key=123123sdfasdf --run-as-client=true 


armv7:
	cargo zigbuild --release --target armv7-unknown-linux-musleabihf
