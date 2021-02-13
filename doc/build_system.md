Build System Design
===================

### Overview

The build system primarily consists of the following files:

* `Makefile` in `tock-on-titan/`.
* `Build.mk` in its subdirectories.

The root Makefile includes `Build.mk` from each directory below it, and each
`Build.mk` includes the `Build.mk`'s in its subdirectories. As a result, all of
the Makefile actions are run from the `tock-on-titan/` directory. Each
`Build.mk`'s actions should contain the path to the `Build.mk` in their name;
for instance, `userspace/h1_tests/Build.mk` should implement the following
actions:

* `userspace/h1_tests/build`
* `userspace/h1_tests/build-signed`
* `userspace/h1_tests/check`
* `userspace/h1_tests/devicetests`
* `userspace/h1_tests/doc`
* `userspace/h1_tests/program`
* `userspace/h1_tests/run`

We want to be able to run `make` from subdirectories of `tock-on-titan/`, such
as `userspace/h1_tests`, but `Build.mk` cannot be run from subdirectories. To
allow `make` to run from subdirectories, each subdirectory containing code
additionally has a "directory shim" Makefile. This Makefile includes
`DirShim.mk` from the root of the repository.

`DirShim.mk` forwards the common targets (listed above) to the root `Makefile`
and `Build.mk` tree. It prepends the path to the current directory to the
target, so that the corresponding `Build.mk` target is run. There are a few
exceptions to this (documented below).

### Build.mk targets

Each Build.mk should support the following targets:

* `$PATH/build`
* `$PATH/build-signed`
* `$PATH/check`
* `$PATH/devicetests`
* `$PATH/doc`
* `$PATH/localtests`

In addition, the Build.mk for full firmware images should support:

* `$PATH/program`
* `$PATH/run`

### Targets handled by DirShim.mk/Makefile

Not all of the [common makefile targets](./make_targets.md) are implemented by
`Build.mk`. The remaining targets are handled directly by `DirShim.mk` or the
top-level `Makefile`, rather than `Build.mk`. They are:

* `all`: These are redirected to `build` by `DirShim.mk` or the top-level
  `Makefile`, so `Build.mk`s don't need to handle it.
* `clean`: clean is generally done repository-wide by removing the `build/`
  directory. However, some directories (such as cargo workspaces) have some
  other cleanup (e.g. removing `Cargo.lock`), and therefore have their own clean
  targets.
* `prtest`: This always runs repository-wide, and is implemented by the
  top-level Makefile.

### Layout of build/

When possible, we build binaries in a directory called `build/` in the root of
the repository.

`build/` should contain a directory structure that mirrors that of the source
repository. However, because many of our targets are built by Cargo, which
benefits from re-using its target directories (to avoid compiling a library for
a target multiple times), it is not a perfect match. Here is the design for the
directory structure of `build/`:

```
build/
    cargo-host/         # Cargo-managed artifacts for the host machine. Uses
                        # elf2tab's toolchain version.
    device_lock         # Lock file used with flock() to prevent concurrent uses
                        # of the device.
    userspace/
        cargo/          # userspace/ Cargo workspace target tree. Uses
                        # libtock-rs's toolchain version.
        h1_tests/       # Non-cargo-managed files specific to h1_tests
        u2f_app/
        libh1/
        ...
    kernel/
        cargo/          # Cargo-managed artifacts for h1 Tock kernels.
                        # Uses tock's toolchain version.
        ...
    third_party/
        chromiumos-ec/  # chromiumos-ec library artifacts.
```
