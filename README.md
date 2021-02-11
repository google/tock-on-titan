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

Download and Configure Rust

```shell
cd tock-on-titan
make setup
```

### Build all boards and apps

```shell
make
```

Note that if one of TANGO_CODESIGNER{,_KEY} is not set, then signed artifacts
will not be created.
