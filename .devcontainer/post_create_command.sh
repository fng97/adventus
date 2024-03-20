#!/bin/bash
set -euxo pipefail

apt-get update && apt-get install -y cmake postgresql-client
cargo install cargo-shuttle
