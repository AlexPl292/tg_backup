# tg_backup
Backup your messages from the Telegram messenger.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.14] - 2021-07-24
### Added
- Support phone call action

## [0.1.13] - 2021-07-24
### Changed
- Do not recreate me.json every time

## [0.1.11] - 2021-07-22
### Added
- Support contact attachment

### Fixed
- Fix previous file parsing if writing to the old file

### Changed
- Backup reuses last file with messages instead of the creating the new one
- Do not recreate members.json every time

## [0.1.9] - 2021-05-10
### Added
- Support geo location
- Support geo live location

## [0.1.8] - 2021-05-07
### Fixed
- Fix homebrew update

## [0.1.7] - 2021-05-07
### Added
- Automate homebrew update

## [0.1.6] - 2021-05-07
### Added
- Automate releases using GitHub actions

## [0.1.5] - 2021-05-05
### Fixed
- Fix issue with failing while reading backup.json

## [0.1.4] - 2021-05-05
### Changed
- Use another approach for protecting multiple processes

## [0.1.3] - 2021-05-05
### Added
- Saving of groups

## [0.1.2] - 2021-04-27
### Added
- Keep previous logs in log folder
- Backup message forwards information
- Backup message reply information
- Add an option to control the amount of running instances

## [0.1.1] - 2021-04-17
### Added
- `--output` option to change the output directory
- `--quite` option to disable output

## [0.1.0] - 2021-04-03

Initial version
