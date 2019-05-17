Common Makefile Targets
=======================

### Overview

For consistency, we want to expose the same `make` targets in each Makefile. For
simplicity of development and testing, we want `make` commands run at the root
of the repository to operate throughout the project. For example, running `make
build` in `tock-on-titan/` should build all targets in the repository.

This specification defines common target names for the `tock-on-titan`
repository. It is meant to be a reference for users of the build system (e.g.
during most development and testing). For documentation of how the build system
works, see [Build System Design](./build_system.md).

### Target list

As most of the project's code is Rust code, many of these targets intentionally
resemble `cargo`'s command line interface. The recursion behavior of required
targets intentionally resembles the behavior of `bazel <action> ...`.

#### Required Targets

Unless otherwise noted, the following targets operate in the directory in which
`make` is invoked as well as its (transitive) subdirectories, similar to `bazel
<action> ...`. All per-directory makefiles support these targets, even if they
do nothing (e.g. `make devicetests` in a directory with no device tests is a
no-op but will succeed):

* `all` is an alias for `build` (following Makefile tradition).
* `build` compiles the code. This can be a library, an application image, the
  kernel image, or the full flash image (kernel + applications).
* `check` runs `cargo check`-style tests.
* `clean` removes all build artifacts we know how to remove. It applies to the
  repository, not just the subdirectory it is called in.
* `devicetests` runs tests that require deploying firmware images.
* `doc` builds documentation.
* `localtests` runs tests that do not require development hardware (e.g. unit
  tests that can run on the host).
* `prtest` depends on the repository-wide `build`, `localtests`, and
  `devicetests`. `prtest` should be done before sending a PR (or PR revision)
  for code review.

#### Binary-Specific Targets

These targets only exist in directories containing a Tock application or
application bundle.

* `program` deploys a firmware image to the dev board but does not connect to
  its console.
* `run` deploys a firmware image and connects to its console to show debug
  output.
