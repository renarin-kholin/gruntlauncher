# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
