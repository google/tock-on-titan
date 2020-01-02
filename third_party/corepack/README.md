# corepack
A better messagepack implementation for serde

[Documentation](https://docs.rs/corepack)

[MPL 2.0 License](LICENSE)

To use:
```toml
corepack = "~0.3.0"
```

If you want to use corepack in a `no_std` environment (nightly rust required),
disable the "std" feature and enable the "alloc" feature:

```toml
corepack = { version = "~0.3.0", default-features = false, features = ["alloc"] }
```

You _must_ choose either "std" or "alloc" as a feature. Corepack currently
requires dynamic allocations in a few situations.
