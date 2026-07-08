# Contributing

## Workflow

This repo follows [GitHub Flow](https://docs.github.com/en/get-started/using-github/github-flow):

1. Branch off `main`.
2. Open a pull request. CI (`cargo fmt`, `cargo clippy --all-targets -- -D warnings`,
   `cargo test`, on Linux and Windows) must pass before merging.
3. Merge via squash merge: each PR becomes a single commit on `main`.

Direct pushes to `main` are disabled by a branch ruleset; all changes go through a PR.

### Branch naming

- `feat/...`: new functionality
- `fix/...`: bug fixes
- `chore/...`: tooling, dependencies, config
- `refactor/...`: internal restructuring with no behavior change

## Commit messages

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org):

```
feat: add mod search pagination
fix: correct windows launch path
chore: bump iced to 0.14
```

Common types: `feat`, `fix`, `chore`, `refactor`, `docs`, `ci`, `test`, `perf`, `style`, `build`.

### Versioning (pre-1.0)

The project is currently `0.x`, so SemVer's pre-1.0 rules apply:

- `feat:` / `fix:`:  patch bump
- `feat!:` or a `BREAKING CHANGE:` footer → minor bump

A change is **breaking** if it requires action from an existing user, or breaks/loses
something they already relied on. In this project that's almost always a persisted
on-disk format: if a change to `Config`, `GruntInstance`, `GameVersionStore`, or similar
means an existing file on a user's disk no longer loads, it's breaking. Adding a new
field with a sane default is not breaking.

### Local commit linting

This repo uses [cocogitto](https://github.com/cocogit-org/cocogitto) to validate commit
messages. After cloning:

```sh
cog install-hook commit-msg
```

This installs a local git hook that rejects non-conventional commit messages before
they're committed.

## Releases

Releases are automated by [release-plz](https://release-plz.dev): merging commits to
`main` updates a standing "Release PR" that bumps the version in `Cargo.toml` and
updates `CHANGELOG.md`. Merging that PR tags and publishes the release. You shouldn't
need to bump versions or write changelog entries by hand.
