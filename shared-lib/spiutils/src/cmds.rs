/// SPI flash op codes
#[repr(u8)]
pub enum OpCodes {
    /// No operation
    Nop = 0x00,

    ////////////////////////////////////////////////////////////
    // Status commands

    /// Returns contents of eeprom_status register.
    /// Implemented in hardware.
    ReadStatusRegister = 0x05,

    /// Disables writes to device, sets WEL = 0 in hardware.
    WriteDisable = 0x04,

    /// Enables writes to device, sets WEL = 1 in hardware.
    WriteEnable = 0x06,

    /// Suspend write. Software should set WSP or WSE = 1.
    WriteSuspend = 0xb0,

    /// Resumes write. Software should set WSP or WSE = 0.
    WriteResume = 0x30,

    ////////////////////////////////////////////////////////////
    // Erase and program commands

    /// Clears bits of a particular 4KB sector to '1'.
    /// Must be implemented in software. HW sets BUSY bit.
    SectorErase = 0x20,

    /// Clears bits of a particular 32KB block to '1'.
    /// Must be implemented in software. HW sets BUSY bit.
    BlockErase32KB = 0x52,

    /// Clears bits of a particular 64KB block to '1'.
    /// Must be implemented in software. HW sets BUSY bit.
    BlockErase64KB = 0xd8,

    /// Clears all bits to '1'.
    /// Must be implemented in software. HW sets BUSY bit.
    ChipErase = 0xc7,

    /// Alternative op code for ChipErase. HW sets BUSY bit.
    ChipErase2 = 0x60,

    /// Programs up to 256 bytes of memory.
    /// Must be implemented in software. HW sets BUSY bit.
    PageProgram = 0x02,

    ////////////////////////////////////////////////////////////
    // ID commands

    /// Retrieves JEDEC-ID as configured in jedec_id registers.
    /// Implemented in hardware.
    ReadJedec = 0x9f,

    /// Retrieves SFDP as configured in sfdp registers.
    /// Implemented in hardware.
    ReadSfdp = 0x5a,

    ////////////////////////////////////////////////////////////
    // Read commands

    /// Retrieves data. The behavior of this command depends on the selected
    /// mode.
    NormalRead = 0x03,

    /// Retrieves data. The behavior of this command depends on the selected
    /// mode. Fast read includes a 1 byte delay after retrieving the last
    /// bit of the addrees before the first bit of data is delivered.
    FastRead = 0x0b,

    /// Similar to FastReads but uses explicit 4 byte addressing.
    FastRead4B = 0x0c,

    /// Similar to FastRead with output on both MISO and MOSI.
    FastReadDualOutput = 0x3b,

    ////////////////////////////////////////////////////////////
    // Address mode commands

    /// Enable 4 byte address mode.
    /// Must be implemented in software.
    Enter4ByteAddressMode = 0xb7,

    /// Disable 4 byte address mode and revert to 3 byte address mode.
    /// Must be implemented in software.
    Exit4ByteAddressMode = 0xe9,
}
