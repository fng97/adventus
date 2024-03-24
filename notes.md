# Notes

## Todos

- add non-test queries after sqlx macros
- add disconnecting after 30s of inactivity
- add dice roller
- add configuration crate, remove hard-coded values
- add proper error handling
- add logging via instrument macros
- don't let it go to sleep (remove shuttle timeout)
- command for adding a track for a user
  - validate URL
  - add postgres (sqlx) database to shuttle project
- check for unused dependencies
- dependabot
- PR to zero2prod for double quoting (shellcheck warnings)
- issue asking if they want a dev container
- pin versions so dev container is faster to build

## Future Improvements

- cache audio
- set up telemetry with Grafana Cloud

- add CI checks
  - tests
  - clippy
  - security
  - shfmt
  - shellcheck
