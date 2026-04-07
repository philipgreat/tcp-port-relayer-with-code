all:
	cargo run

armv7-musl:
	cargo zigbuild --release --target armv7-unknown-linux-musleabihf
