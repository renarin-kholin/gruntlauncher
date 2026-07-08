## Summary

<!-- What does this change do, and why? -->

## Type of change

- [ ] `feat`/`fix`: patch or minor bump (pre-1.0)
- [ ] `feat!` / `BREAKING CHANGE`: minor bump (pre-1.0). Breaks an on-disk format
      (config/instance/version files no longer load), removes user-facing
      functionality, or otherwise requires action from an existing user.
- [ ] `chore`/`refactor`/`docs`/`ci`: no release

## Checklist

- [ ] `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings` pass locally
- [ ] If this touches a persisted struct (`Config`, `GruntInstance`, `GameVersionStore`,
      etc.): existing files on disk still load, or this is marked as breaking above
