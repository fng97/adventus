# Notes

## Todos

- add configuration crate, remove hard-coded values
- add proper error handling
- add logging via instrument macros
- don't let it go to sleep (remove shuttle timeout)
- command for adding a track for a user
  - validate URL
  - add postgres (sqlx) database to shuttle project
- check for unused dependencies
- PR to zero2prod for double quoting (shellcheck warnings)

## Future Improvements

- cache audio
- set up telemetry with Grafana Cloud

- add CI checks
  - tests
  - clippy
  - security
  - shfmt
  - shellcheck
