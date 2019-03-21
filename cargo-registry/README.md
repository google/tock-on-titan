We vendor our dependencies; building this project should not require making
network requests (beyond fetching the source and installing the toolchain).
Cargo needs an index to operate, so we maintain this local index of our vendored
dependencies. We cannot use `third_party/` directly because it contains
submodules and therefore isn't in the format that cargo expects.

To vendor packages, use
[cargo-vendor](https://github.com/alexcrichton/cargo-vendor). IMPORTANT: do not
tell it to vendor into `third_party/` directly. Instead, have it create a new
directory and manually copy over the vendored packages. `cargo-vendor` is happy
to delete existing files and directories, and because `third_party/` contains
git submodules you could potentially lose non-backed-up work. After copying the
`cargo-vendor`-created package to `third_party/`, manually create a simlink to
it in this directory.

When possible, we should refer to vendored dependencies using `path`
dependencies rather than using this registry. However, using the registry is
necessary for dependencies of crates in submodules as well as crates in the
registry.
