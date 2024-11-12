# Contributing guide

This document contains some rules you should adhere to when contributing to this repository.

## GitHub

* Disclose your potential contribution beforehand on GitHub by [creating a new discussion item](https://github.com/edgeless-project/edgeless/discussions/new/choose).
* When working on a new feature / issue, create a branch from the GitHub issue
  and add your changes there. To merge the changes into the main, create a pull
  request and assign someone as a reviewer. The reviewer should then reject or
  accept the changes / leave some comments. After the changes are accepted by
  the reviewer, he should take care to merge them and remove the dangling
  feature branch.
* Do not introduce merge commits on the main branch. Merges to the main branch
  must be fast-forwarded. A good practice is also to squash the commits on the
  feature branch (this can be done while merging on GitHub).

## Code

* Follow a "no use" policy, with the exception of traits.
* Run the rust formatter before committing (`cargo fmt`). This ensures we
  minimize the noise coming from, e.g., whitespace changes.
* Try to limit the number of warnings (ideally, there should not be any
  warnings). A good way to do this is to run `cargo fix` before running the
  formatter. Suggested workflow:
```bash
  cargo fix --allow-staged --allow-dirty && cargo fmt && git commit`
```
* gRPC protobuf message definitions:
  * Provide detailed comments for all the fields.
  * Never reuse field numbers.
  * You may use incremental field numbers, starting from 1, or group field numbers in
    chunks (tens of hundreds) to allow future fields to be added to a "chunk" which they
    semantically belong to.

## Licensing

* When making a significant contribution to a file in the repository:
  1. Add yourself he [list of contributors](CONTRIBUTORS.txt) and adhere to the license.
  2. Add one `SPDX-FileCopyrightText` line to the beginning of the file, as the last
     one before the line with `SPDX-License-Identifier`
* Do not taint this repository with incompatible licenses: everything not MIT-licensed
  must be kept external to this repository.

## Releasing

When creating a new release follow the steps below:

- Run `scripts/add_spdx_headers.sh` from each crates to update the SPDX headers.
- Update relevant document in [README.md](README.md) and [documentation](documentation).
- Use [semantic versioning](https://semver.org/) for the new tag.
- Update [CHANGELOG.md](CHANGELOG.md) following the guidelines [here](https://keepachangelog.com/).
- Make sure that
  - New contributors have been added to `CONTRIBUTORS.txt`.
  - All tests and checks succeed with `cargo test`, `cargo fmt --check`, and
    `cargo clippy`.
  - All the functions build correctly with `scripts/functions_build.sh`.
  - All the examples work with `scripts/run_all_examples.sh`.