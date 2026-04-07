#!/bin/sh

set -eu

usage() {
    echo "Usage: $0 <management_base_url> <auth_key>" >&2
    echo "Example: $0 http://relay.example.com:8080 987654321" >&2
}

if [ "$#" -ne 2 ]; then
    usage
    exit 1
fi

management_base_url=$1
auth_key=$2

management_base_url=${management_base_url%/}

client_ip=$(curl -fsS "$management_base_url/ip")

if command -v sha256sum >/dev/null 2>&1; then
    key=$(printf '%s' "${client_ip}${auth_key}" | sha256sum | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
    key=$(printf '%s' "${client_ip}${auth_key}" | shasum -a 256 | awk '{print $1}')
elif command -v openssl >/dev/null 2>&1; then
    key=$(printf '%s' "${client_ip}${auth_key}" | openssl dgst -sha256 | awk '{print $NF}')
else
    echo "sha256 tool not found: need one of sha256sum, shasum, openssl" >&2
    exit 1
fi

auth_url="${management_base_url}/${key}"

echo "client_ip=${client_ip}"
echo "auth_url=${auth_url}"
curl -fsS "${auth_url}"
