# Notes

## Todos

- don't let it go to sleep (remove shuttle timeout)
- command for adding a track for a user
  - validate URL
  - add postgres (sqlx) database to shuttle project
- check for unused dependencies
- move logic into separate modules
- cache audio
- set up telemetry with Grafana Cloud
- get dev container working again (have to call docker from docker)

- environment setup
  - had to install `shfmt` with `apt`
    - use [feature](https://github.com/devcontainers-contrib/features/tree/main/src/shfmt)?
  - `cargo install sqlx-cli --no-default-features --features postgres`
  - `nix-env -i postgresql`
  - `nix-env -i yt-dlp`
- add CI checks
  - shfmt
  - shellcheck
