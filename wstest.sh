#!/usr/bin/env bash

docker run --rm --interactive --tty \
  -v "$PWD/config:/config" \
  -v "$PWD/reports:/reports" \
  -p 9001:9001 \
  --name fuzzingserver \
  crossbario/autobahn-testsuite wstest --debug --mode=fuzzingserver --spec=/config/fuzzingserver.json
