#!/bin/bash

docker build -f Dockerfile-arm-build -t fly-rs-arm64-build .
docker run -v "$PWD/:/fly.rs" fly-rs-arm64-build /bin/bash -c "source ~/.cargo/env && OPENSSL_DIR=/root/openssl cargo build --target=aarch64-unknown-linux-gnu --release"
