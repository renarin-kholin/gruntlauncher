# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.2](https://github.com/renarin-kholin/gruntlauncher/compare/v0.2.1...v0.2.2) (2026-07-18)


### Bug Fixes

* **updater:** update progress not visible ([#37](https://github.com/renarin-kholin/gruntlauncher/issues/37)) ([f47a7e3](https://github.com/renarin-kholin/gruntlauncher/commit/f47a7e3f5820935741fc93b16b6f4d429a839877))

## [0.2.1](https://github.com/renarin-kholin/gruntlauncher/compare/v0.2.0...v0.2.1) (2026-07-17)


### Features

* remove instance ([#32](https://github.com/renarin-kholin/gruntlauncher/issues/32)) ([d33280a](https://github.com/renarin-kholin/gruntlauncher/commit/d33280af575274162f8a20cb1d5f54e5eabce895))

## [0.2.0](https://github.com/renarin-kholin/gruntlauncher/compare/v0.1.9...v0.2.0) (2026-07-16)


### ⚠ BREAKING CHANGES

* edit instance screen ([#30](https://github.com/renarin-kholin/gruntlauncher/issues/30))

### Features

* edit instance screen ([#30](https://github.com/renarin-kholin/gruntlauncher/issues/30)) ([f0ab353](https://github.com/renarin-kholin/gruntlauncher/commit/f0ab353486ffd3409d0ce8faf97976cde568f094))

## [0.1.9](https://github.com/renarin-kholin/gruntlauncher/compare/v0.1.8...v0.1.9) (2026-07-13)


### Features

* verify downloads ([#27](https://github.com/renarin-kholin/gruntlauncher/issues/27)) ([e247744](https://github.com/renarin-kholin/gruntlauncher/commit/e2477443c403a27e2318884459e87d568c89300f))

## [0.1.8](https://github.com/renarin-kholin/gruntlauncher/compare/v0.1.7...v0.1.8) (2026-07-13)


### Features

* add update progress ([#25](https://github.com/renarin-kholin/gruntlauncher/issues/25)) ([ff355e7](https://github.com/renarin-kholin/gruntlauncher/commit/ff355e7e3455d7273ecfff91efe4804e8415ad01))

## [0.1.7](https://github.com/renarin-kholin/gruntlauncher/compare/v0.1.6...v0.1.7) (2026-07-13)


### Features

* add settings ([#23](https://github.com/renarin-kholin/gruntlauncher/issues/23)) ([35f784d](https://github.com/renarin-kholin/gruntlauncher/commit/35f784d43e6950aa8895ada56c609faece89c08d))

## [0.1.6](https://github.com/renarin-kholin/gruntlauncher/compare/v0.1.5...v0.1.6) (2026-07-12)


### Features

* add velopack auto updates ([#19](https://github.com/renarin-kholin/gruntlauncher/issues/19)) ([a219bee](https://github.com/renarin-kholin/gruntlauncher/commit/a219bee437f5512d43bd299365bef13c3ecf0717))

## [0.1.5](https://github.com/renarin-kholin/gruntlauncher/compare/v0.1.4...v0.1.5) (2026-07-12)


### Bug Fixes

* **ci:** stop dist from creating the github release ([#17](https://github.com/renarin-kholin/gruntlauncher/issues/17)) ([dbddbbd](https://github.com/renarin-kholin/gruntlauncher/commit/dbddbbdded13c2b381edba39197c530ebe92f2de))

## [0.1.4](https://github.com/renarin-kholin/gruntlauncher/compare/v0.1.3...v0.1.4) (2026-07-12)


### Bug Fixes

* minor bugs ([#13](https://github.com/renarin-kholin/gruntlauncher/issues/13)) ([061f303](https://github.com/renarin-kholin/gruntlauncher/commit/061f303e0b73d1e185769445f48580ba31d4a876))

## [Unreleased]

## [0.1.3](https://github.com/renarin-kholin/gruntlauncher/releases/tag/v0.1.3) - 2026-07-12

Note: this version was bumped manually. A prior manual edit to `Cargo.toml`
left the version "stuck" at 0.1.2 (already tagged), which made release-plz
treat it as already released and stop computing further bumps from new
commits. See `release-plz.toml`/`release-plz.yml` for the fix that prevents
this going forward.

### Added

- *(ui)* Add more buttons on sidebar

### Fixed

- *(ci)* Match a proven release-plz + dist setup

## [0.1.2](https://github.com/renarin-kholin/gruntlauncher/releases/tag/v0.1.2) - 2026-07-11

### Added

- *(auth)* Add login for vintage story accounts ([#5](https://github.com/renarin-kholin/gruntlauncher/pull/5))
- install game on windows via the silent installer with registry cleanup
- download selected mods during instance installation
- browse and select ModDB mods in the add-instance wizard
- load local versions instead of redownloading if they exist
- download, install, and launch game instances
- cache game versions and add local/remote version sources
- add game version selection and instance persistence
- add loading config and instances
- add open in browser
- move to a custom blitz webview crate
- *(add_instance)* add reviews page
- add webview for mod info
- add mod selection view
- convert screens to overlays
- add home and add instance views

### Fixed

- *(config)* change load config from sync to async
- *(ci)* use a fine-grained PAT so release-plz's tag push triggers dist
- incorrect launch command on windows
- platform specific settings

### Other

- bump version to 0.1.2
- bump version to 0.1.1, skipping v0.1.0
- *(gruntlauncher)* release v0.1.0 ([#4](https://github.com/renarin-kholin/gruntlauncher/pull/4))
- add dist for windows/linux release binaries ([#3](https://github.com/renarin-kholin/gruntlauncher/pull/3))
- *(ci)* setup Github Flow and release automation ([#2](https://github.com/renarin-kholin/gruntlauncher/pull/2))
- *(ci)* setup Github Flow and release automation
- use shared image handle for grunt icon instead of creating a handle every time
- add README.md and LICENSE
- create FUNDING.yml
- remove unnecessary platform specific method
- extract shared helpers and simplify screen lifecycle
- update checkout action
- cargo clippy fix
- update packages
- remove mold from project config
- fmt
- add ci yaml
- minor ui changes
- minor ui changes and hook in instance creation message
- initial commit

## [0.1.1](https://github.com/renarin-kholin/gruntlauncher/releases/tag/v0.1.1) - 2026-07-11

Note: v0.1.0 was skipped — GitHub's immutable-release feature permanently
reserved that tag name after an earlier failed release attempt, with no
release actually published under it.

### Added

- *(auth)* Add login for vintage story accounts ([#5](https://github.com/renarin-kholin/gruntlauncher/pull/5))
- install game on windows via the silent installer with registry cleanup
- download selected mods during instance installation
- browse and select ModDB mods in the add-instance wizard
- load local versions instead of redownloading if they exist
- download, install, and launch game instances
- cache game versions and add local/remote version sources
- add game version selection and instance persistence
- add loading config and instances
- add open in browser
- move to a custom blitz webview crate
- *(add_instance)* add reviews page
- add webview for mod info
- add mod selection view
- convert screens to overlays
- add home and add instance views

### Fixed

- incorrect launch command on windows
- platform specific settings

### Other

- add dist for windows/linux release binaries ([#3](https://github.com/renarin-kholin/gruntlauncher/pull/3))
- *(ci)* setup Github Flow and release automation ([#2](https://github.com/renarin-kholin/gruntlauncher/pull/2))
- *(ci)* setup Github Flow and release automation
- use shared image handle for grunt icon instead of creating a handle every time
- add README.md and LICENSE
- create FUNDING.yml
- remove unnecessary platform specific method
- extract shared helpers and simplify screen lifecycle
- update checkout action
- cargo clippy fix
- update packages
- remove mold from project config
- fmt
- add ci yaml
- minor ui changes
- minor ui changes and hook in instance creation message
- initial commit
