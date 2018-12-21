#ifndef __HID_DFU_H__
#define __HID_DFU_H__

enum dfu_err {
  DFU_SUCCESS = 0,
  DFU_WRONG_STATE = 1,           // uninitialized?
  DFU_OUT_OF_SEQUENCE = 1 << 1,  // out-of-order update block
  DFU_BAD_MAGIC_NO = 1 << 2,
  DFU_BAD_ADDRESS = 1 << 3,    // update address didn't match expected
  DFU_NOT_MONOTONIC = 1 << 4,  // non-monotonic update
  DFU_FLASH_ERROR = 1 << 5,
};

int u2fhid_cmd_DFU(const uint8_t *buf, const uint16_t bcnt);

#endif  // __HID_DFU_H__
