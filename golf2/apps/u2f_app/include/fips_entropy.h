#ifndef __FIPS_ENTROPY_H
#define __FIPS_ENTROPY_H

void flash_info_read_enable(uint32_t addr, uint32_t size);
void flash_info_read_enable(uint32_t addr, uint32_t size) {}

uint32_t flash_physical_info_read_word(uint32_t addr);
uint32_t flash_physical_info_read_word(uint32_t addr) {
  return fips_entropy[addr];
}

#endif
