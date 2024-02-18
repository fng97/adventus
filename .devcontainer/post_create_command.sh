#!/bin/bash
set -euxo pipefail

apt-get update && apt-get install -y cmake python3-venv

python3 -m venv .venv
source .venv/bin/activate

pip install --upgrade pip
pip install yt-dlp
