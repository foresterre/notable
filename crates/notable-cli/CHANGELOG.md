# Changelog

## Unreleased

### Added

* Added the `notable` CLI, which can be installed with `cargo install notable-cli`
* Added `notable lint`, which checks whether a changelog follows the keep-a-changelog format
* Added `notable reorder`, which re-orders the change kind sections of every release
* `notable reorder --order` sets the order of the sections, kinds which aren't listed go last
* `notable reorder --write` writes the re-ordered changelog back, instead of printing it
* Added `notable merge-conflicts`, which suggests how to solve merge conflicts in a changelog
* `notable merge-conflicts --resolve` writes the resolved changelog back
* `notable merge-conflicts --diff` prints the suggestions as a diff, which can be applied with `git apply`
* `notable merge-conflicts --interactive` asks for each conflict whether to apply its resolution
* Changelog sections may use custom change kinds, like `### Performance`
