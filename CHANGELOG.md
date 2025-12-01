# Changelog

## [1.1.4](https://github.com/gianged/gianged-attendance/compare/v1.1.3...v1.1.4) (2025-12-01)


### Bug Fixes

* add .gitattributes for consistent CRLF on Windows ([993f5bd](https://github.com/gianged/gianged-attendance/commit/993f5bd20bec929345d08d1f541596a70a405ff4))

## [1.1.3](https://github.com/gianged/gianged-attendance/compare/v1.1.2...v1.1.3) (2025-12-01)


### Bug Fixes

* normalize line endings to CRLF for Inno Setup on Windows ([f60e5a9](https://github.com/gianged/gianged-attendance/commit/f60e5a93e347d020359bc7afe4a18c6b96e50478))

## [1.1.2](https://github.com/gianged/gianged-attendance/compare/v1.1.1...v1.1.2) (2025-12-01)


### Bug Fixes

* use Swatinem/rust-cache for better caching ([aced709](https://github.com/gianged/gianged-attendance/commit/aced709c33374537b10f6e3ffdc8929f7018915c))

## [1.1.1](https://github.com/gianged/gianged-attendance/compare/v1.1.0...v1.1.1) (2025-12-01)


### Bug Fixes

* update gianged-attendance version to 1.1.0 in Cargo.lock ([3e60b2b](https://github.com/gianged/gianged-attendance/commit/3e60b2bc50fcb3a60dee211a48178b037386359c))
* update setup.iss version in build workflow instead of ([b60ff33](https://github.com/gianged/gianged-attendance/commit/b60ff33e837f8c69a730b55bfe913d8796b6f17c))

## [1.1.0](https://github.com/gianged/gianged-attendance/compare/v1.0.0...v1.1.0) (2025-12-01)


### Features

* add CI/CD pipeline with semantic versioning ([ca074ac](https://github.com/gianged/gianged-attendance/commit/ca074ac13ace79ce8f888a8eb90320a28c7efa49))
* Add logging initialization and cleanup functionality; include new dependencies for tracing ([090dd9a](https://github.com/gianged/gianged-attendance/commit/090dd9a34da2acdbb551c859d7ef508e9aec96d5))
* Add main application UI and implement dashboard, department, staff, and sync panels ([15989c2](https://github.com/gianged/gianged-attendance/commit/15989c272b4bf0f73d9b37e8a270a786f425878f))
* Enhance attendance management with detailed reporting and settings ([44df719](https://github.com/gianged/gianged-attendance/commit/44df71951fde08e34fae0b432e41fced90d9f515))
* Enhance dashboard with additional reports card and update navigation layout ([1ac61f7](https://github.com/gianged/gianged-attendance/commit/1ac61f7bc22a7f8633cae53038286b579de22ea7))
* Enhance date input handling in reports panel with validation and synchronization ([f45c697](https://github.com/gianged/gianged-attendance/commit/f45c69711d07dbdb2340429ce6668cd70f072264))
* Implement attendance and employee repositories with CRUD operations and sync service ([f6a11ec](https://github.com/gianged/gianged-attendance/commit/f6a11ec16a95fa2ae346f6c3d7e7eb9c6625ef58))
* Remove unused .gitkeep file and add note icon ([48ccfde](https://github.com/gianged/gianged-attendance/commit/48ccfdee449b4fd4ed95b3da6d532b107a70c6d9))
* Rename device_uid to scanner_uid across the application for consistency ([93e506b](https://github.com/gianged/gianged-attendance/commit/93e506bc2bf6c5e941907586ae008c664867f46e))


### Bug Fixes

* use explicit jsonpath for Cargo.toml version updates ([23dc1cc](https://github.com/gianged/gianged-attendance/commit/23dc1cc48fe90f8795b1a941e5c834acc041ea39))


### Refactoring

* Rename project from GiangEd to Gianged Attendance ([17c2a2b](https://github.com/gianged/gianged-attendance/commit/17c2a2b49d89533f879f676ce10638f3c568f773))
* Update string formatting to use interpolation for improved readability ([5ea1040](https://github.com/gianged/gianged-attendance/commit/5ea1040e66e29aee3d114e7cd288c428abe23286))
