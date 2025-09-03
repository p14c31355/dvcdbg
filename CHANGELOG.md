## [0.3.0] - 2025-09-03

### Added

- Implementing a HAL error-compatible general trait
- Serial error handling
- Scanner.rs error handling
- Change error definition to nested enum
- Delete logger
- Implement display trait for ErrorKind
- I2C scan error DRY handling
- Support #88 #72 #69
- Implementation UART marker trait
- Integrated uart backend to embedded-io
- Separate backend embedded-io and nb
- Implement explorer.rs
- Logging Sequence discovery progress
- Implementation gluing explorer to scanner.rs
- Add compat/ascii.rs
- Completely abandons linear search and adopts topological sorting by in-degree
- Delete dupulication function
- Separate logger
- Change API name & add wrapper macros

### Chores

- *(release)* Update changelog and version to 0.3.0

### Fix

- #94 handle staging method
- #94 Fixed an issue where the search tree could not be reverted
- #94 Implement traits that can flexibly accommodate protocol-specific knowledge

### Fixed

- Up LOG_BUFFER_CAPACITY
- Eliminate the causes of garbled text and panic

### Docs

- Edit README.md & USAGE.md

### Explorer.rs

- Fix lifetime

## [0.2.1] - 2025-08-20

### Added

- Implement WriteAdapter
- Delete unnecessary feature
- Bridge to Arduino

### Chores

- *(release)* Update changelog and version to

### Docs

- Edit README.md: Usage Example
- Edit README.md: Integrated USAGE.md
- Edit README.md: Edit Quickstart
- Edit USAGE.md
- Edit README.md: Edit Quickstart

## [0.2.0] - 2025-08-19

### Added

- Implementing a macro that absorbs HAL generic types
- Implementation of internal macros and interface macros
- #41 PR review respond & Diversification of compatible models
- #43 Boundary conditions are no longer required.
- Support HAL trait path for examples revision
- Support embedded-hal,io latest
- Embedded-hal 1.0.0 support
- Embedded-hal 1.0.0 support
- Adapt_serial macro into add attribute branching
- Adapt_serial macro into add attribute branching
- Refactoring scanner.rs
- Scanner.rs into add attribute branching
- Refactoring scanner.rs
- Refactoring scanner.rs
- Refactoring macros.rs
- Adapt_serial macro supports embedded-hal 1.0

### Chores

- *(release)* Update changelog and version to
- HAL compatibility support
- Support latest HAL trait path
- Support latest HAL trait path?
- Support latest HAL trait path?
- Support latest HAL trait path?

### Fix

- The avr-hal Usart requires the UsartOps<H, RX, TX> trait boundary for generic type U, but the macro-generated UsartAdapter<U, RX, TX, CLOCK> does not have the boundary, resulting in a compile error.
- Macro failed to resolve
- Macro failed to resolve
- Macro failed to resolve
- #43 fix Type parameters are not expanded in the impl block.
- The trait boundary of generic U in the macro does not match the MCU type.

## [0.1.2] - 2025-08-14

### Added

- #36 Add adapter boilerplate generation macros
- #37 Add useful macros
- #37 Add binary writing macro
- #37 Edit rustdoc
- #37 Add macro workflow
- #37 Edit rustdoc
- Update README.md (quickstart section)

### Chores

- *(release)* Update changelog and version to
- Features Organised
- Docs: Edit README.md

### Fix

- Delete outdated feature

### Fixed

- #38 review respond: parts moduling & parts features
## [0.1.1] - 2025-08-14

### Chores

- Settings infrastructions
- Edit release.yml & touch merge.yml
- Added automatic correction for everyday use and behaviour during merging.
- Edit CONTRIBUTING.md
- Doc transrate
- Separate clippy step and auto test step in autofix.yml & Delete push merged commit step in merge.yml & fix clippy friendly in scanner.rs
- Infra: Avoid loop
- Infra: Settings trusted-publish
- Infra: Implement release.yml draft
- Infra: Defaulting common processing
- Format: #34 responded
- Format: #32 responded
- Format: #31 responded & moved .git-cliff.toml

### Fixed

- Fix clippy warnings
- CI: Delete measure binary size process
- CI: Fix lost change issue

### ToDo

- Fix compile errors
