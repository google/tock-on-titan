Google Cloud Build
==================

### Overview

Tock-on-Titan uses [Cloud Build](https://cloud.google.com/cloud-build) for test
infrastructure.

Tock-on-Titan uses a "toolchain container" uploaded into [Container
Registry](https://cloud.google.com/container-registry) as its build toolchain.
This image should be rebuilt manually whenever the Rust toolchains used by
Tock-on-Titan change (e.g. after a major `tock` or `libtock-rs` update).

### Rebuilding the Toolchain Image

To rebuild the toolchain image, run (in `tock-on-titan/`):

```
gcloud builds submit --config gcb/tock-toolchain.yaml gcb
```
