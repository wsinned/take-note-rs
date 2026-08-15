# Changelog
All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

- - -
## v2.4.0 - 2026-08-15
#### Features
- add Nix flake installation - (ba9cff7) - Dennis Woodruff
#### Continuous Integration
- validate Nix flake - (290c97d) - Dennis Woodruff
#### Miscellaneous Chores
- raise Rust baseline to 1.92 - (6795ce4) - Dennis Woodruff

- - -

## v2.3.7 - 2026-07-17
#### Documentation
- record Windows CLI test isolation gap - (b69167d) - Dennis Woodruff
- record test coverage gaps - (bdeb985) - Dennis Woodruff
#### Tests
- cover weekly CLI workflow - (1fe1f0b) - Dennis Woodruff

- - -

## v2.3.6 - 2026-07-16
#### Bug Fixes
- raise MSRV for let chains - (74714c1) - Dennis Woodruff
#### Documentation
- remove invalid library examples - (6a7983a) - Dennis Woodruff
#### Tests
- add CLI integration coverage - (e3c4fd5) - Dennis Woodruff
#### Continuous Integration
- upgrade release tool cache action - (47e0785) - Dennis Woodruff
#### Miscellaneous Chores
- define platform and MSRV contract - (9986588) - Dennis Woodruff

- - -

## v2.3.5 - 2026-07-16
#### Bug Fixes
- reject invalid config section structures - (aabfe69) - Dennis Woodruff

- - -

## v2.3.4 - 2026-07-13
#### Bug Fixes
- recover from malformed config - (72abf4b) - Dennis Woodruff
#### Documentation
- track cache action upgrade - (351927f) - Dennis Woodruff

- - -

## v2.3.3 - 2026-07-12
#### Continuous Integration
- cache pinned release tools - (cd4fa65) - Dennis Woodruff

- - -

## v2.3.2 - 2026-07-12
#### Bug Fixes
- prevent concurrent note truncation - (a8bd7b4) - Dennis Woodruff
#### Continuous Integration
- consolidate release pipeline - (3f1518c) - Dennis Woodruff

- - -

## v2.3.1 - 2026-07-12
#### Bug Fixes
- preserve note permissions - (3abc2c9) - Dennis Woodruff
#### Miscellaneous Chores
- strengthen contributor checks - (4cdbc6c) - Dennis Woodruff

- - -

## v2.3.0 - 2026-07-11
#### Features
- add insert mode - (6f630b1) - Dennis Woodruff
- add markdown heading helper - (fde627d) - Dennis Woodruff
#### Bug Fixes
- (**markdown**) remove redundant dead_code suppressions - (0dd2b76) - Dennis Woodruff
- missing insert mode in test initialisers - (a06caa3) - Dennis Woodruff
#### Miscellaneous Chores
- add conventional commit hook - (9664adc) - Dennis Woodruff

- - -

## v2.2.0 - 2026-07-08
#### Features
- add take-note init setup wizard - (2255264) - Dennis Woodruff, *Claude Sonnet 4.6*
#### Bug Fixes
- replace results[0] with results.first() to make invariant explicit - (b1cbde9) - Dennis Woodruff, *Claude Sonnet 4.6*
- reject --config default with a clear error message - (89e24cb) - Dennis Woodruff, *Claude Sonnet 4.6*
- warn on stderr when editor binary cannot be launched - (5dee8fe) - Dennis Woodruff, *Claude Sonnet 4.6*
- percent-encode file path in Obsidian URI - (0777e0c) - Dennis Woodruff, *Claude Sonnet 4.6*
#### Documentation
- add git hooks setup instructions to Development section - (6c8e168) - Dennis Woodruff, *Claude Sonnet 4.6*
- sync README with current implementation - (8389515) - Dennis Woodruff, *Claude Sonnet 4.6*
#### Refactoring
- extract merge_over_default to eliminate duplicated merge logic - (d4d0a1f) - Dennis Woodruff, *Claude Sonnet 4.6*

- - -

## v2.1.0 - 2026-06-16
#### Features
- add headless append mode - (9f4738c) - Dennis Woodruff
#### Bug Fixes
- use standard temp file persistence - (2b39789) - Dennis Woodruff

- - -

## v2.0.7 - 2026-06-16
#### Miscellaneous Chores
- tidy low-risk code nits - (be65b2b) - Dennis Woodruff

- - -

## v2.0.6 - 2026-06-14
#### Miscellaneous Chores
- (**ci**) use Node 24 release actions - (332cd32) - Dennis Woodruff

- - -

## v2.0.5 - 2026-06-13
#### Bug Fixes
- build release binaries from version tag - (3fc3f44) - Dennis Woodruff

- - -

## v2.0.4 - 2026-06-13
#### Bug Fixes
- sync package version during releases - (2fe765a) - Dennis Woodruff
- verify assets before publishing releases - (c8c37ae) - Dennis Woodruff

- - -

## v2.0.3 - 2026-06-13
#### Bug Fixes
- preserve platform names for release assets - (72034cb) - Dennis Woodruff

- - -

## v2.0.2 - 2026-06-13
#### Bug Fixes
- configure linux arm64 cross compilation - (28192e8) - Dennis Woodruff

- - -

## v2.0.1 - 2026-06-13
#### Bug Fixes
- configure git identity for version bumps - (a2832d2) - Dennis Woodruff
- correct cocogitto v7 configuration - (0eb99fc) - Dennis Woodruff
#### Documentation
- update README with eget install and project context - (65659e7) - Dennis Woodruff
#### Continuous Integration
- add conventional commit versioning and multi-platform releases - (5f658c3) - Dennis Woodruff
- restrict build job to main branch only - (1cea9e5) - Dennis Woodruff
#### Miscellaneous Chores
- fix cocogitto hook format - (054280b) - Dennis Woodruff
- add cocogitto config for conventional commits - (fb79f7c) - Dennis Woodruff
#### Style
- fix formatting from pre-commit hook - (fe0a0a2) - Dennis Woodruff

- - -

Changelog generated by [cocogitto](https://github.com/cocogitto/cocogitto).