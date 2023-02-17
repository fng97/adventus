# Adventus

A (hopefully) fun Discord bot.

## Usage

Adventus can be added to your server using [this link](https://discord.com/api/oauth2/authorize?client_id=1074795024946036889&permissions=134144&scope=bot).

### Dice Roller

Use XdY format to roll X dice with Y sides. Returns the value of each die.

For example, `!roll 2d20` returns something like: "`@user` rolled 13, 7."

## Improvements

- [ ] Move the roller to a lambda function.
  - [ ] Define infra in CloudFormation.
  - [ ] Document the infra.
  - [ ] Re-write it in CDK for Python.
  - [ ] Re-write the lambda in Rust.
- [ ] Add dependency management.
- [ ] Version development environment.
  - [ ] Extensions to use.
  - [ ] Extension workspace settings.
- [ ] Add CI/CD.
  - [ ] Style
  - [ ] Quality
  - [ ] Dependencies
  - [ ] Deployment
