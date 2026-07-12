# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
