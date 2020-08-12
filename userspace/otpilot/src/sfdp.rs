use spiutils::protocol::flash::AddressMode;

pub enum SfdpTableError {
    TargetLenTooSmall,
}

pub fn get_table(
    data: &mut[u8],
    image_size_bits : u32,
    startup_address_mode : AddressMode,
    support_address_mode_switch : bool,
    mailbox_offset: u32,
    mailbox_size: u32,
    google_capabilities: u32) -> Result<(), SfdpTableError> {

    // JESD216A
    let sfdp : [u8; 104] = [
        // SFDP Header 1st DWORD
        0x53, // S
        0x46, // F
        0x44, // D
        0x50, // P


        // SFDP Header 2nd DWORD
        0x05, // Minor (=JESD216A)
        0x01, // Major (=JESD216A)
        0x01, // # parameter headers (1=2x header)
        0xff, // unused


        // Basic Flash Parameter header v1.5, 16DWs starting at DW6
        // Parameter Header 1st DWORD
        0x00, // ID LSB (=Basic Parameter Table)
        0x05, // Table Minor (=JESD216)
        0x01, // Table Major (=JESD216A)
        0x10, // Table Length (16 DWORDs)


        // Parameter Header 2nd DWORD
        0x18, 0x00, 0x00, // Table Pointer (=0x000018)
        0xFF, // ID MSB (=JEDEC)


        // Google (MFG ID 0x26 in Bank 9) parameter header v1.0, 4DWs starting at DW22
        // Parameter Header 1st DWORD
        0x26, // ID LSB (=Basic Parmaeter Table)
        0x00, // Table Minor (=JESD216)
        0x01, // Table Major (=JESD216A)
        0x04, // Table Length (4 DWORDs)


        // Parameter Header 2nd DWORD
        0x58, 0x00, 0x00, // Table Pointer (=0x000058)
        0x09, // ID MSB (=Bank 9)


        // Basic Flash Parameter Table v1.0 1st DWORD
        // <1:0>   : Block/Sector Erase granularity available for the entirety of flash:
        //            - 0x1 if 4KiB is uniformly available
        //            - 0x3 if 4KiB is unavailable
        0x1 << 0 |  // 4KiB erase uniformly available
        // <2>     : Write granularity (0 if the buffer is less than 64B, 1 if larger)
        0x1 << 2 |  // page size is 64 or larger
        // <3>     : Write Enable Instruction Required for writing to Volatile Status
        //           Register:
        //            - 0x0 if target flash only has nonvolatile status bits and does
        //              not require status register to be written every power on
        //            - 0x1 if target flash requires 0x00 to be written to the status
        //              register in order to allow writes and erases
        0x0 << 3 |  // nonvolatile only status register
        // <4>     : Write Enable Opcode Select for Writing to Volatile Status Register:
        //            - 0x0 if 0x50 is the opcode to enable a status register write
        //            - 0x1 if 0x06 is the opcode to enable a status register write
        0x1 << 4 |  // 0x06 write enable for status register
        // <7:5>   : Unused
        0x0 << 5,

        // <15:8>  : 4KiB Erase Opcode (0xFF if unsupported)
        0x20,  // 4KiB erase opcode

        // <16>    : Supports 1-1-2 Fast Read (1 if supported)
        0x0 << 0 |  // 1-1-2 is not supported
        // <18:17> : Address Bytes:
        //            - 0x0 if 3 Byte addressing only
        //            - 0x1 if defaults to 3B addressing, enters 4B on command
        //            - 0x2 if 4 Byte addressing only
        (match startup_address_mode {
            AddressMode::ThreeByte => if support_address_mode_switch { 1 } else { 0 }
            AddressMode::FourByte => 2
        }) << 1 |
        // <19>    : Supports Double Transfer Rate (DTR) Clocking (1 if supported)
        0x0 << 3 |  // DTR clocking not supported
        // <20>    : Supports 1-2-2 Fast Read (1 if supported)
        0x0 << 4 |  // 1-2-2 not supported
        // <21>    : Supports 1-4-4 Fast Read (1 if supported)
        0x0 << 5 |  // 1-1-4 not supported
        // <22>    : Supports 1-1-4 Fast Read (1 if supported)
        0x0 << 6 |  // 1-4-4 not supported
        // <23>    : Unused
        0x0 << 7,
        // <31:24> : Unused
        0x0,


        // Basic Flash Parameter Table v1.0 2nd DWORD
        // <30:0> : N, where:
        //           - if =< 2 gibibits, flash memory density is N+1 bits
        //           - if > 2 gibibits, flash memory density is 2^N bits
        ((image_size_bits >> 0) & 0xff) as u8,
        ((image_size_bits >> 8) & 0xff) as u8,
        ((image_size_bits >> 16) & 0xff) as u8,
        ((image_size_bits >> 24) & 0x7f) as u8 |
        // <31>   : Density greater than 2 gibibits
        0 << 7,


        // Basic Flash Parameter Table v1.0 3rd DWORD
        // ------------------------------------------
        // <4:0>   : 1-4-4 Fast Read Number of Wait States (Dummy CLocks)
        // <7:5>   : 1-4-4 Fast Read Number of Mode Bits (0 if unsupported)
        0x0, // 1-4-4 is not supported
        // <15:8>  : 1-4-4 Fast Read Opcode
        0x0, // 1-4-4 is not supported
        // <20:16> : 1-1-4 Fast Read Number of Wait States (Dummy Clocks)
        // <23:21> : 1-1-4 Fast Read Number of Mode Bits (0 if unsupported)
        0x0, // 1-1-4 is not supported
        // <31:24> : 1-1-4 Fast Read Opcode
        0x0, // 1-1-4 is not supported


        // Basic Flash Parameter Table v1.0 4th DWORD
        // ------------------------------------------
        // <4:0>   : 1-1-2 Fast Read Number of Wait States (Dummy CLocks)
        // <7:5>   : 1-1-2 Fast Read Number of Mode Bits (0 if unsupported)
        0x8, // 8 dummy cycles
        // <15:8>  : 1-1-2 Fast Read Opcode
        0x3b,
        // <20:16> : 1-2-2 Fast Read Number of Wait States (Dummy Clocks)
        // <23:21> : 1-2-2 Fast Read Number of Mode Bits (0 if unsupported)
        0x0, // 1-2-2 is not supported
        // <31:24> : 1-2-2 Fast Read Opcode
        0x0, // 1-2-2 is not supported


        // Basic Flash Parameter Table v1.0 5th DWORD
        // ------------------------------------------
        // <0>    : Supports 2-2-2 Fast Read (1 if supported)
        // <3:1>  : Reserved (0x7)
        // <4>    : Supports 4-4-4 Fast Read (1 if supported)
        // <31:5> : Reserved (0x7FFFFFF)
        0xee, 0xff, 0xff, 0xff, // 4-4-4 and 2-2-2 are not supported


        // Basic Flash Parameter Table v1.0 6th DWORD
        // ------------------------------------------
        // <31:24> : 2-2-2 Fast Read Opcode
        // <23:21> : 2-2-2 Fast Read Number of Mode Bits (0 if unsupported)
        // <20:16> : 2-2-2 Fast Read Number of Wait States (Dummy Clocks)
        // <15:0>  : Reserved (0xFFFF)
        0xff, 0xff, 0x00, 0x00, // 2-2-2 is not supported


        // Basic Flash Parameter Table v1.0 7th DWORD
        // ------------------------------------------
        // <31:24> : 4-4-4 Fast Read Opcode
        // <23:21> : 4-4-4 Fast Read Number of Mode Bits (0 if unsupported)
        // <20:16> : 4-4-4 Fast Read Number of Wait States (Dummy Clocks)
        // <15:0>  : Reserved (0xFFFF)
        0xff, 0xff, 0x00, 0x00, // 4-4-4 is not supported


        // Basic Flash Parameter Table v1.0 8th DWORD
        // ------------------------------------------
        // <7:0>   : Sector Type 1 Erase Size (2^N Bytes, 0 if unavailable)
        12, // 4 KiB
        // <15:8>  : Sector Type 1 Erase Opcode
        0x20,
        // <23:16> : Sector Type 2 Erase Size (2^N Bytes, 0 if unavailable)
        0, // unavailable
        // <31:24> : Sector Type 2 Erase Opcode
        0, // unavailable


        // Basic Flash Parameter Table v1.0 9th DWORD
        // ------------------------------------------
        // <7:0>   : Sector Type 3 Erase Size (2^N Bytes, 0 if unavailable)
        0, // unavailable
        // <15:8>  : Sector Type 3 Erase Opcode
        0, // unavailable
        // <23:16> : Sector Type 4 Erase Size (2^N Bytes, 0 if unavailable)
        0, // unavailable
        // <31:24> : Sector Type 4 Erase Opcode
        0, // unavailable


        // Basic Flash Parameter Table v1.5 10th DWORD
        // ------------------------------------------
        // 128ms typical 4KiB erase time, 512ms max (2 * (1 + 1) * 128).
        // MX25L25635FMI: 43ms typical, 200ms max (sector erase).
        // W25Q256FV: 45ms typical, 400ms max (sector erase).
        //
        // <3:0>   : Multiplier from typical to maximum erase time, where
        //           maximum_time = 2// (multiplier + 1)// typical_time
        1 << 0 |
        // <8:4>   : Sector Type 1 Erase, Typical time count, where
        //           time = (count + 1)// units
        0 << 4,
        // <10:9>  : Sector Type 1 Erase, Typical time units, where
        //           0x0: 1ms, 0x1: 16ms, 0x2: 128ms, 0x3: 1s
        2 << 1 | // 128 ms
        // <15:11> : Sector Type 2 Erase, Typical time count, where
        //           time = (count + 1)// units
        0 << 3, // unavailable
        // <17:16> : Sector Type 2 Erase, Typical time units, where
        //           0x0: 1ms, 0x1: 16ms, 0x2: 128ms, 0x3: 1s
        // <22:18> : Sector Type 3 Erase, Typical time count, where
        //           time = (count + 1)// units
        // <24:23> : Sector Type 3 Erase, Typical time units, where
        //           0x0: 1ms, 0x1: 16ms, 0x2: 128ms, 0x3: 1s
        0x0, // unavailable
        // <29:25> : Sector Type 4 Erase, Typical time count, where
        //           time = (count + 1)// units
        // <31:30> : Sector Type 4 Erase, Typical time units, where
        //           0x0: 1ms, 0x1: 16ms, 0x2: 128ms, 0x3: 1s
        0x0, // unavailable


        // Basic Flash Parameter Table v1.5 11th DWORD
        // ------------------------------------------
        // <3:0>   : Multiplier from typical time to max time for programming, where
        //           maximum_time = 2// (multiplier + 1)// typical_time
        1 << 0 |
        // <7:4>   : Page Size (2^N Bytes)
        8 << 4, // 256B page size
        // 1 mS for page program
        // <12:8>  : Page Program, Typical time count, where time = (count + 1)// units
        0xf << 0 |
        // <13>    : Page Program, Typical time units (0: 8us, 1: 64us)
        1 << 5 |
        // 128 uS for first byte written
        // <17:14> : First Byte Program, Typical time count, where each byte takes
        //           time = (count + 1)// units// bytes
        3 << 6,
        3 << 0 |
        // <18>    : First Byte Program, Typical time units (0: 1us, 1: 8us)
        1 << 2 |
        // 128 uS per additional byte written
        // <22:19> : Additional Byte Program, Typical time count, where each byte takes
        //           time = (count + 1)// units// bytes. This should not be
        //           used if the additional bytes count exceeds 1/2 a page size.
        0xf << 3 |
        // <23>    : Additional Byte Program, Typical time units (0: 1us, 1: 8us)
        1 << 7,
        // chip erase takes 128-512 seconds
        // <28:24> : Chip Erase, Typical time count, where time = (count + 1)// units
        1 << 0 |
        // <30:29> : Chip Erase, Typical time units, where
        //           0x0: 16ms, 0x1: 256ms, 0x2: 4s, 0x3: 64s
        3 << 5 |
        // <31>    : Reserved (0x1)
        1 << 7,


        // Basic Flash Parameter Table v1.5 12th DWORD
        // ------------------------------------------
        // <3:0>   : Prohibited Operations During Program Suspend flags, where
        //           xxx0b May not initiate a new erase anywhere
        //                 (erase nesting not permitted)
        //           xxx1b May not initiate a new erase in the program suspended page
        //                 size
        //           xx0xb May not initiate a new page program anywhere
        //                 (program nesting not permitted)
        //           xx1xb May not initiate a new page program in the program suspended
        //                 page size
        //           x0xxb Refer to vendor datasheet for read restrictions
        //           x1xxb May not initiate a read in the program suspended page size
        //           0xxxb Additional erase or program restrictions apply
        //           1xxxb The erase and program restrictions in bits 1:0 are
        //                 sufficient
        // <7:4>   : Prohibited Operations During Erase Suspend flags, where
        //           xxx0b May not initiate a new erase anywhere
        //                 (erase nesting not permitted)
        //           xxx1b May not initiate a new erase in the erase suspended sector
        //                 size
        //           xx0xb May not initiate a page program anywhere
        //           xx1xb May not initiate a page program in the erase suspended
        //                 sector size
        //           x0xxb Refer to vendor datasheet for read restrictions
        //           x1xxb May not initiate a read in the erase suspended sector size
        //           0xxxb Additional erase or program restrictions apply
        //           1xxxb The erase and program restrictions in bits 5:4 are
        //                 sufficient
        0x0,
        // <8>     : Reserved (0x1)
        // <12:9>  : Program resume to suspend minimum internal, (count + 1)// 64us
        // <17:13> : Suspend in-progress program max latency count, where
        //           max latency = (count + 1)// units
        1 << 0, // reserved
        // <19:18> : Suspend in-progress program max latency units, where
        //           0x0: 128ns, 0x1: 1us, 0x2: 8us, 0x3: 64us
        // <23:20> : Erase resume to suspend minimum interval, (count + 1)// 64us
        0x0,
        // <28:24> : Suspend in-progress erase max latency count, where
        //           max latency = (count + 1)// units
        // <30:29> : Suspend in-progress erase max latency units, where
        //           0x0: 128ns, 0x1: 1us, 0x2: 8us, 0x3: 64us
        // <31>    : Suspend / Resume unsupported (1 unsupported, 0 supported)
        1 << 7, // unsupported


        // Basic Flash Parameter Table v1.5 13th DWORD
        // ------------------------------------------
        // <7:0>   : Program Resume Instruction used to resume a program operation
        0x00,
        // <15:8>  : Program Suspend Instruction used to suspend a program operation
        0x00,
        // <23:16> : Resume Instruction used to resume a write or erase type operation
        0x00,
        // <31:24> : Suspend Instruction used to suspend a write or erase type operation
        0x00,


        // Basic Flash Parameter Table v1.5 14th DWORD
        // ------------------------------------------
        // <1:0>   : Reserved (0x3)
        3 << 0 |
        // <7:2>   : Status Register Polling Device Busy Flags, where
        //           xx_xx1xb Bit 7 of the Flag Status Register may be polled any time
        //                    a Program, Erase, Suspend/Resume command is issued, or
        //                    after a Reset command while the device is busy. The read
        //                    instruction is 70h. Flag Status Register bit definitions:
        //                    bit[7]: Program or erase controller status
        //                    (0=busy; 1=ready)
        //           xx_xxx1b Use of legacy polling is supported by reading the Status
        //                    Register with 05h instruction and checking WIP bit[0]
        //                    (0=ready; 1=busy).
        1 << 2, // Use legacy status register polling for WIP bit
        // <12:8>  : Exit deep powerdown to next operation delay count, where
        //           delay = = (count + 1)// units
        // <14:13> : Exit deep powerdown to next operation delay units, where
        //           0x0: 128ns, 0x1: 1us, 0x2: 8us, 0x3: 64us
        // <22:15> : Exit deep powerdown instruction
        // <30:23> : Enter deep powerdown instruction
        // <31>    : Deep powerdown unsupported (1 unsupported, 0 supported)
        0x0, 0x0, 1 << 7, // unsupported


        // Basic Flash Parameter Table v1.5 15th DWORD
        // ------------------------------------------
        // <3:0>   : 4-4-4 mode disable sequences, where
        //           xxx1b issue FFh instruction
        //           xx1xb issue F5h instruction
        //           x1xxb device uses a read-modify-write sequence of operations:
        //                 read configuration using instruction 65h followed by address
        //                 800003h, clear bit 6,
        //                 write configuration using instruction 71h followed by
        //                 address 800003h. This configuration is volatile.
        //           1xxxb issue the Soft Reset 66/99 sequence
        // <8:4>   : 4-4-4 mode enable sequences, where
        //           x_xxx1b set QE per QER description above, then issue
        //                   instruction 38h
        //           x_xx1xb issue instruction 38h
        //           x_x1xxb issue instruction 35h
        //           x_1xxxb device uses a read-modify-write sequence of operations:
        //                   read configuration using instruction 65h followed by
        //                   address 800003h, set bit 6,
        //                   write configuration using instruction 71h followed by
        //                   address 800003h. This configuration is volatile.
        // <9>     : 0-4-4 mode supported (1 supported, 0 unsupported)
        // <15:10> : 0-4-4 Mode Exit Method, where
        //           xx_xxx1b Mode Bits[7:0] = 00h will terminate this mode at the end
        //                    of the current read operation
        //           xx_xx1xb If 3-Byte address active, input Fh on DQ0-DQ3 for 8
        //                    clocks. If 4-Byte address active, input Fh on DQ0-DQ3 for
        //                    10 clocks. This will terminate the mode prior to the next
        //                    read operation.
        //           xx_1xxxb Input Fh (mode bit reset) on DQ0-DQ3 for 8 clocks. This
        //                    will terminate the mode prior to the next read operation.
        // <19:16> : 0-4-4 Mode Entry Method, where
        //           xxx1b Mode Bits[7:0] = A5h Note: QE must be set prior to using this
        //                 mode
        //           xx1xb Read the 8-bit volatile configuration register with
        //                 instruction 85h, set XIP bit[3] in the data read, and write
        //                 the modified data using the instruction 81h, then Mode Bits
        //                 [7:0] = 01h
        // <22:20> : Quad Enable Requirements (1-1-4, 1-4-4, 4-4-4 Fast Reads), where
        //           000b Device does not have a QE bit. Device detects 1-1-4 and 1-4-4
        //                reads based on instruction. DQ3/HOLD# functions as hold during
        //                instruction phase.
        //           001b QE is bit 1 of status register 2. It is set via Write Status
        //                with two data bytes where bit 1 of the second byte is one. It
        //                is cleared via Write Status with two data bytes where bit
        //                1 of the second byte is zero. Writing only one byte to the
        //                status register has the side-effect of clearing status
        //                register 2, including the QE bit. The 100b code is used if
        //                writing one byte to the status register does not modify status
        //                register 2.
        //           010b QE is bit 6 of status register 1. It is set via Write Status
        //                with one data byte where bit 6 is one. It is cleared via Write
        //                Status with one data byte where bit 6 is zero.
        //           011b QE is bit 7 of status register 2. It is set via Write status
        //                register 2 instruction 3Eh with one data byte where bit 7 is
        //                one. It is cleared via Write status register 2 instruction
        //                3Eh with one data byte where bit 7 is zero. The status
        //                register 2 is read using instruction 3Fh.
        //           100b QE is bit 1 of status register 2. It is set via Write Status
        //                with two data bytes where bit 1 of the second byte is one. It
        //                is cleared via Write Status with two data bytes where bit 1
        //                of the second byte is zero. In contrast to the 001b code,
        //                writing one byte to the status register does not modify status
        //                register 2.
        //           101b QE is bit 1 of the status register 2. Status register 1 is
        //                read using Read Status instruction 05h. Status register 2 is
        //                read using instruction 35h. QE is set via Write Status
        //                instruction 01h with two data bytes where bit 1 of the second
        //                byte is one. It is cleared via Write Status with two data
        //                bytes where bit 1 of the second byte is zero.
        // <23>    : HOLD and WIP disable supported by setting the non-volatile extended
        //           configuration register's bit 4 to 0.
        0x0, 0x0, 0x0, // unsupported
        // <31:24> : Reserved (0xFF)
        0xff,


        // Basic Flash Parameter Table v1.5 16th DWORD
        // -------------------------------------------
        // <6:0>   : Volatile or Non-Volatile Register and Write Enable Instruction for
        //           Status Register 1, where
        //           xx0_0000b status register is read only
        //           xxx_xxx1b Non-Volatile Status Register 1, powers-up to last written
        //                     value, use instruction 06h to enable write
        //           xxx_xx1xb Volatile Status Register 1, status register powers-up
        //                     with bits set to "1"s, use instruction 06h to enable
        //                     write
        //           xxx_x1xxb Volatile Status Register 1, status register powers-up
        //                     with bits set to "1"s, use instruction 50h to enable
        //                     write
        //           xxx_1xxxb Non-Volatile/Volatile status register 1 powers-up to last
        //                     written value in the non-volatile status register, use
        //                     instruction 06h to enable write to non-volatile status
        //                     register. Volatile status register may be activated after
        //                     power-up to override the non-volatile status register,
        //                     use instruction 50h to enable write and activate the
        //                     volatile status register.
        //           xx1_xxxxb Status Register 1 contains a mix of volatile and
        //                     non-volatile bits. The 06h instruction is used to enable
        //                     writing of the register.
        0x2 << 0 | // Volatile status reg, powers up with bits set to 1, use
                   // 0x06 to write enable
        // <7>     : Reserved (0x1)
        1 << 7,
        // <13:8>  : Soft Reset and Rescue Sequence Support, where
        //           00_0000b no software reset instruction is supported
        //           xx_xxx1b drive Fh on all 4 data wires for 8 clocks
        //           xx_xx1xb drive Fh on all 4 data wires for 10 clocks if device is
        //                    operating in 4-byte address mode
        //           xx_x1xxb drive Fh on all 4 data wires for 16 clocks
        //           xx_1xxxb issue instruction F0h
        //           x1_xxxxb issue reset enable instruction 66h, then issue reset
        //                    instruction 99h. The reset enable, reset sequence may be
        //                    issued on 1, 2, or 4 wires depending on the device
        //                    operating mode.
        //           1x_xxxxb exit 0-4-4 mode is required prior to other reset sequences
        //                    above if the device may be operating in this mode.
        0x0 << 0 | // no software reset instruction supported
        // <23:14> : Exit 4-Byte Addressing, where
        //           xx_xxxx_xxx1b issue instruction E9h to exit 4-Byte address mode
        //                         (write enable instruction 06h is not required)
        if support_address_mode_switch { 1 << 6 } else { 0 },
        //           xx_xxxx_xx1xb issue write enable instruction 06h, then issue
        //                         instruction E9h to exit 4-Byte address mode
        //           xx_xxxx_x1xxb 8-bit volatile extended address register used to
        //                         define A[31:A24] bits. Read with instruction C8h.
        //                         Write instruction is C5h, data length is 1 byte.
        //                         Return to lowest memory segment by setting A[31:24]
        //                         to 00h and use 3-Byte addressing.
        //           xx_xxxx_1xxxb 8-bit volatile bank register used to define A[30:A24]
        //                         bits. MSB (bit[7]) is used to enable/disable 4-byte
        //                         address mode. When MSB is cleared to ‘0’, 3-byte
        //                         address mode is active and A30:A24 are used to select
        //                         the active 128 Mbit memory segment. Read with
        //                         instruction 16h. Write instruction is 17h, data
        //                         length is 1 byte.
        //           xx_xxx1_xxxxb A 16-bit nonvolatile configuration register controls
        //                         3-Byte/4-Byte address mode. Read instruction is B5h.
        //                         Bit[0] controls address mode [0=3-Byte; 1=4-Byte].
        //                         Write configuration register instruction is B1h, data
        //                         length is 2 bytes.
        //           xx_xx1x_xxxxb Hardware reset
        if startup_address_mode == AddressMode::ThreeByte { 1 << 3 } else { 0 } |
        //           xx_x1xx_xxxxb Software reset (see bits 13:8 in this DWORD)
        //           xx_1xxx_xxxxb Power cycle
        if startup_address_mode == AddressMode::ThreeByte { 1 << 5 } else { 0 },
        // <31:24> : Enter 4-Byte Addressing, where
        //           xxxx_xxx1b issue instruction B7h
        //                      (preceding write enable not required)
        //           xxxx_xx1xb issue write enable instruction 06h, then issue
        //                      instruction B7h
        //           xxxx_x1xxb 8-bit volatile extended address register used to define
        //                      A[31:24] bits. Read with instruction C8h. Write
        //                      instruction is C5h with 1 byte of data. Select the
        //                      active 128 Mbit memory segment by setting the
        //                      appropriate A[31:24] bits and use 3-Byte addressing.
        //           xxxx_1xxxb 8-bit volatile bank register used to define A[30:A24]
        //                      bits. MSB (bit[7]) is used to enable/disable 4-byte
        //                      address mode. When MSB is set to ‘1’, 4-byte address
        //                      mode is active and A[30:24] bits are don’t care. Read
        //                      with instruction 16h. Write instruction is 17h with 1
        //                      byte of data. When MSB is cleared to ‘0’, select the
        //                      active 128 Mbit segment by setting the appropriate
        //                      A[30:24] bits and use 3-Byte addressing.
        //           xxx1_xxxxb A 16-bit nonvolatile configuration register controls
        //                      3-Byte/4-Byte address mode. Read instruction is B5h.
        //                      Bit[0] controls address mode [0=3-Byte; 1=4-Byte]. Write
        //                      configuration register instruction is B1h, data length
        //                      is 2 bytes.
        //           xx1x_xxxxb Supports dedicated 4-Byte address instruction set.
        //                      Consult vendor data sheet for the instruction set
        //                      definition.
        //           x1xx_xxxxb Always operates in 4-Byte address mode
        if startup_address_mode == AddressMode::FourByte {
                1 << 6 // Always operates in 4-Byte address mode
        } else {
                if support_address_mode_switch {
                    1  // issue instruction B7h
                } else {
                    0
                }
        },

        // Google parameter table
        0x47, // G
        0x4f, // O
        0x4f, // O
        0x47, // G

        ((mailbox_offset >> 0) & 0xff) as u8,
        ((mailbox_offset >> 8) & 0xff) as u8,
        ((mailbox_offset >> 16) & 0xff) as u8,
        ((mailbox_offset >> 24) & 0xff) as u8,

        ((mailbox_size >> 0) & 0xff) as u8,
        ((mailbox_size >> 8) & 0xff) as u8,
        ((mailbox_size >> 16) & 0xff) as u8,
        ((mailbox_size >> 24) & 0xff) as u8,

        ((google_capabilities >> 0) & 0xff) as u8,
        ((google_capabilities >> 8) & 0xff) as u8,
        ((google_capabilities >> 16) & 0xff) as u8,
        ((google_capabilities >> 24) & 0xff) as u8,
    ];

    if data.len() < sfdp.len() {
        return Err(SfdpTableError::TargetLenTooSmall);
    }

    for idx in 0..sfdp.len() {
        data[idx] = sfdp[idx];
    }
    for idx in sfdp.len()..data.len() {
        data[idx] = !0;
    }

    Ok(())
}
