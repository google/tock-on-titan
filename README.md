# Tock-on-Titan

This repository contains ports of Tock OS (https://www.tockos.org) to Titan
chips.

This is not an officially supported Google product.


## Getting started

### Clone the repo

Get the source:

```shell
git clone --recursive https://github.com/google/tock-on-titan.git
```

### Get the tools and libs to build the code

Download Rust

```shell
cd tock-on-titan
curl https://sh.rustup.rs -sSf | sh
```

Configure Rust

```shell
make setup
```

### Build all boards and apps (unsigned)

```shell
make build
```

### Build 'signed' versions of all artifacts

```shell
make build-signed
```

The `build-signed` target requires `TANGO_CODESIGNER` and `TANGO_CODESIGNER_KEY`
to be set. The codesigner and keys are not publicly available.
