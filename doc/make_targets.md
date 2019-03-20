Common Makefile Targets
=======================

### Overview

For simplicity of development and testing, we want `make` commands run at the
root of the repository to recurse through and operate on all binaries within the
project. For example, running `make test` in `tock-on-titan/` should run all
available tests in the repository.

In order to do this, we need the higher-level Makefiles to recurse into
lower-level Makefiles. This requires the subdirectory Makefiles to have
similar targets -- i.e. "test" should have similar behavior in all
subdirectories.

This specification defines the common targets that, when reasonable, all
Makefiles in `tock-on-titan` should support.

### Target list

As most of the project's code is Rust code, many of these targets resemble
`cargo`'s command line interface.

* `all` is an alias for `build` (following Makefile tradition).
* `build` compiles the code. This can be a library, an application image, the
  kernel image, or the full flash image (kernel + applications). `build` should
  operate recursively.
* `check` runs `cargo check`-style tests, and should operate recursively.
* `clean` removes all build artifacts we know how to remove, and should operate
  recursively.
* `doc` builds documentation (where available), and should operate recursively.
* `program` deploys a firmware image to the dev board but does not connect to
  its console. `program` should not operate recursively, as it does not make
  sense to deploy multiple programs simultaneously.
* `run` deploys a firmware image and connects to its console to show debug
  output. It should not operate recursively.
* `test` runs any available tests. In the future, this may need to be split
  between test types/mechanisms. For example, we may need tests that run on the
  developer's machine (platform-independent) to be separate from tests that
  require a dev board. This should operate recursively, but tests that require
  deploying firmware images must run sequentially.

Makefiles should implement the sensible subset of the above list. For example, a
Makefile for a C application most likely should not implement `check` or `doc`,
and a code library should not implement `program` or `run`. Like Bazel, trying
to build an unsupported recursive target should result in no action (e.g.
"nothing to test") rather than an error.
