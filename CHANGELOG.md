# Changelog

## Unreleased

### Added

* Added the `notable-merge` crate, which finds and resolves merge conflicts in a changelog
* Sections may now use custom change kinds, like `### Performance`
* Added `notable::reorder_sections`, which re-orders the sections of a changelog and keeps the rest of the text as is
* Added `Changelog::reorder_sections`, which re-orders the sections of a parsed changelog
* Added `ChangeKind::position_in`, which finds the position of a kind in a given order

### Changed

* ⚠️ Breaking change: Renamed `ChangeKind::ALL` to `ChangeKind::STANDARD`
* ⚠️ Breaking change: `ChangeKind` is no longer `Copy`, since it can now hold a custom name
* ⚠️ Breaking change: `ChangeKind::as_str` now takes `&self`
* ⚠️ Breaking change: Parsing a `ChangeKind` now only fails for an empty name, with `EmptyChangeKind`
* ⚠️ Breaking change: A section with an empty heading is now reported as `SectionError::MissingName`

### Removed

* ⚠️ Breaking change: Removed `UnknownChangeKind` and `SectionError::UnknownKind`, since any section name is now accepted

## 0.1.0 - 2026-11-12

### Added

* Added the `notable` crate, a parser for changelogs which follow the keep-a-changelog format
* Added the `notable-core` crate, which contains the types of a parsed changelog
