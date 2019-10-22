---
driver number: 0x80040000
---

NvCounter System Calls
======================

## Overview

The NvCounter driver provides a global, non-volatile, atomically-incremented,
anti-rollback counter. The counter must be initialized (to a value of 0) by
kernel code, but can then be incremented by userspace.

## Command

  * ### Command number: `0`

    ** Description**: Indicates whether the NvCounter driver is available.

    **Argument 1**: unused

    **Argument 2**: unused

    **Returns**: `SUCCESS` if the NvCounter is available, and `ENODEVICE` if it
    is not available.

  * ### Command number: `1`

    **Description**: Reads and increments the counter. The read and increment
    run asynchronously, and the result is sent to subscribe number `0`.

    **Argument 1**: unused

    **Argument 2**: unused

    **Returns**: `ENODEVICE` if NvCounter is not available, `EBUSY` if this app
    has already scheduled an increment, `EFAIL` if flash initialization failed,
    and `SUCCESS` otherwise.

## Subscribe

  * ### Subscribe number: `0`

    **Description**: Read-and-increment results. This callback is run when an
    increment option completes.

    **Callback signature**: The callback receives two arguments. The first is
    `0` if the read failed, `1` if the read succeeded, and `2` if the read and
    increment succeeded. If the read succeeded, the second argument is the
    current counter value.

    **Returns**: `SUCCESS` if the subscribe was successful, and `EINVAL` if the
    app is somehow invalid.
