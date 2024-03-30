# Notes

## Todos

1. [x] add YT url validation
2. [x] add disconnecting the client after 30s of inactivity
3. [x] pass clones of reqwest and PgPool instead of references
4. [x] add more tests
5. [ ] add counter metrics to postgres for now (transaction?)
   1. [ ] dice rolls
   2. [ ] introductions
6. [x] add proper error handling
   1. [x] ask ChatGPT to review this
7. [ ] configuration module: remove hard-coded values?
8. [ ] UPDATE README!

- PR to zero2prod for double quoting (shellcheck warnings)
  - issue asking if they want a dev container
- pin versions so dev container is faster to build
- start the database from rust when testing or running locally
- document changes to roll

### CI

- add code coverage with tarpaulin
- add security checks with cargo deny (cargo-crev?)
- check for unused dependencies
- dependabot
- tests
- clippy
- security
- shfmt
- shellcheck

### CD

- automate deployment on push to main with Shuttle
- don't let it go to sleep (remove shuttle timeout) (CD)

## Future Improvements

- cache audio
- instrument with tracing spans
- help command
