# Changelog

## [1.3.0](https://github.com/gianged/gianged-attendance/compare/v1.2.1...v1.3.0) (2025-12-25)


### Features

* add database and UI module files with initial structure ([1f777ab](https://github.com/gianged/gianged-attendance/commit/1f777ab4bb25668ee3ecec31ac8000cffe8692ee))

## [1.2.1](https://github.com/gianged/gianged-attendance/compare/v1.2.0...v1.2.1) (2025-12-24)


### Bug Fixes

* update cargo lock ([42d2c87](https://github.com/gianged/gianged-attendance/commit/42d2c87aff6c5e7d9640f331b27e08ffa02f150c))
* Update installer to reference renamed database script ([b93fb3a](https://github.com/gianged/gianged-attendance/commit/b93fb3a060ded394105c857816a9b4546a2a85e6))

## [1.2.0](https://github.com/gianged/gianged-attendance/compare/v1.1.5...v1.2.0) (2025-12-24)


### Features

* Add support for ACK and DATA responses in ZK TCP client protocol ([d7a54f8](https://github.com/gianged/gianged-attendance/commit/d7a54f8c296c7e6db4fbca75bc0f08af8ec22455))
* Enhance chunk request handling to support direct DATA responses or ACK followed by DATA ([719dda5](https://github.com/gianged/gianged-attendance/commit/719dda53dc3df62fc9da65b71fdba190b1ab2adf))
* Implement ZK TCP protocol client for attendance data retrieval ([63a4cc1](https://github.com/gianged/gianged-attendance/commit/63a4cc122c30f3f4bdc0c987e72206d4dd111550))
* Improve size query handling by skipping delayed ACK_OK responses and ensuring CMD_ACK_DATA is received ([91586fe](https://github.com/gianged/gianged-attendance/commit/91586fe9e5a823a0ca38533b2634f2e4e1cef2f0))
* Refactor attendance data retrieval to improve chunk reading and handle total size response ([efcf4f2](https://github.com/gianged/gianged-attendance/commit/efcf4f217e9e172e81f40388a50a163cf3f49b60))
* Refactor attendance data retrieval to streamline chunk reading and handle end-of-data conditions ([e839360](https://github.com/gianged/gianged-attendance/commit/e8393608b8626de7c698f109c4a151652cc61709))
* Update attendance record parsing to align with TCP protocol format and correct offsets for user ID and timestamp ([21b6765](https://github.com/gianged/gianged-attendance/commit/21b6765a23f1d6085d4aa718daff9c4b21286d93))
* Update attendance record timestamp handling to use local time instead of UTC ([1910c22](https://github.com/gianged/gianged-attendance/commit/1910c2201d1b26b766fe826a123e8ceab1b30800))
* Update attendance size retrieval to handle ACK_OK response and extract total size correctly ([f8a5d00](https://github.com/gianged/gianged-attendance/commit/f8a5d00bfc4f5a33fd6d96b20a9f202e4c7ae4a7))
* Update dependencies in Cargo.toml ([1e69f22](https://github.com/gianged/gianged-attendance/commit/1e69f22f7d23b3abb99b509fe694b342249848d9))
* Update ZK TCP protocol constants to include CMD_ACK_DATA and adjust CMD_ACK_OK value ([a642fca](https://github.com/gianged/gianged-attendance/commit/a642fca3205fa0ab5151c46c8466a53fdee141c3))

## [1.1.5](https://github.com/gianged/gianged-attendance/compare/v1.1.4...v1.1.5) (2025-12-01)


### Bug Fixes

* update version to 1.1.4 in Cargo.lock and clean up Inno Setup script ([0977029](https://github.com/gianged/gianged-attendance/commit/0977029d09b74436cf16e2f6e76b911cea06bc71))

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
