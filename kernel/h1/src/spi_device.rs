use crate::hil::spi_device::AddressConfig;
use crate::hil::spi_device::SpiDevice;
use crate::hil::spi_device::SpiDeviceClient;

use core::cmp::min;

use kernel::common::cells::OptionalCell;
use kernel::common::registers::register_bitfields;
use kernel::common::registers::register_structs;
use kernel::common::registers::ReadOnly;
use kernel::common::registers::ReadWrite;
use kernel::common::registers::WriteOnly;
use kernel::common::StaticRef;
use kernel::ReturnCode;

use spiutils::protocol::flash::AddressMode;
use spiutils::protocol::flash::OpCode;

// Registers for the SPI device controller
register_structs! {
    Registers {
        // SPI device control register
        (0x0000 => ctrl: ReadWrite<u32, CTRL::Register>),

        /// This word is transmitted when the TX FIFO is empty.
        (0x0004 => dummy_word: ReadWrite<u8>),
        (0x0005 => _reserved0005),

        (0x0008 => status0l: ReadWrite<u8>),
        (0x0009 => status0h: ReadWrite<u8>),
        (0x000a => status1: ReadWrite<u16>),
        (0x000c => status2: ReadWrite<u16>),
        (0x000e => status3: ReadWrite<u16>),
        (0x0010 => status4: ReadWrite<u16>),
        (0x0012 => status5: ReadWrite<u16>),
        (0x0014 => status6: ReadWrite<u16>),
        (0x0016 => status7: ReadWrite<u16>),
        (0x0018 => ctrl0: ReadWrite<u16>),
        (0x001a => ctrl1: ReadWrite<u16>),
        (0x001c => ctrl2: ReadWrite<u16>),
        (0x001e => ctrl3: ReadWrite<u16>),
        (0x0020 => ctrl4: ReadWrite<u16>),
        (0x0022 => ctrl5: ReadWrite<u16>),
        (0x0024 => ctrl6: ReadWrite<u16>),
        (0x0026 => ctrl7: ReadWrite<u16>),

        /// FIFO control register
        (0x0028 => fifo_ctrl: ReadWrite<u32, FIFO_CTRL::Register>),
        /// The number of bytes in the TX FIFO.
        (0x002c => txfifo_size: ReadOnly<u32, TXFIFO_SIZE::Register>),
        /// The current byte read pointer of the TX FIFO. The MSB is used to
        /// detect if the TX FIFO is empty or full.
        (0x0030 => txfifo_rptr: ReadWrite<u32, TXFIFO_RPTR::Register>),
        /// The current byte write pointer of the TX FIFO. The MSB is used to
        /// detect if the TX FIFO is empty or full.
        (0x0034 => txfifo_wptr: ReadWrite<u32, TXFIFO_WPTR::Register>),
        /// TXFIFO_LVL interrupt will be asserted when TXFIFO_SIZE is less than
        /// or equal to this value.
        (0x0038 => txfifo_threshold: ReadWrite<u32, TXFIFO_THRESHOLD::Register>),
        /// The number of 8-bit words in the RX FIFO.
        (0x003c => rxfifo_size: ReadOnly<u32, RXFIFO_SIZE::Register>),
        /// The current byte read pointer of the RX FIFO. The MSB is used to
        /// detect if the RX FIFO is empty or full.
        (0x0040 => rxfifo_rptr: ReadWrite<u32, RXFIFO_RPTR::Register>),
        /// The current byte write pointer of the RX FIFO. The MSB is used to
        /// detect if the RX FIFO is empty or full.
        (0x0044 => rxfifo_wptr: ReadWrite<u32, RXFIFO_WPTR::Register>),
        /// Level of RX FIFO + 1 in 8-bit words for RXFIFO_LVL interrupt
        (0x0048 => rxfifo_threshold: ReadWrite<u32, RXFIFO_THRESHOLD::Register>),
        (0x004c => _reserved004c),


        /// Interrupt state
        (0x0054 => istate: ReadOnly<u32, INTERRUPT::Register>),
        /// Clear interrupts
        (0x0058 => istate_clr: ReadWrite<u32, ISTATE_CLR::Register>),
        (0x005c => _reserved005c),
        /// Enable interrupts
        (0x0064 => ictrl: ReadWrite<u32, INTERRUPT::Register>),
        (0x0068 => _reserved0068),


        /// EEPROM Mode control register
        (0x0400 => eeprom_ctrl: ReadWrite<u32, EEPROM_CTRL::Register>),

        /// The read opcode for accessing generic mailbox mode
        (0x0404 => mailbox_rd_opcode: ReadWrite<u8>),
        (0x0405 => _reserved0405),
        /// The opcode for fast dual reads
        (0x0408 => fast_dual_rd_opcode: ReadWrite<u8>),
        (0x0409 => _reserved0409),
        /// Arbitrary opcodes that cause BUSY to assert
        (0x040c => busy_opcode: [ReadWrite<u32, BUSY_OPCODE::Register>; 8]),


        /// EEPROM status register. The allocation and function of most bits are
        /// not defined in hardware, and is left up to software insetad. The
        /// only exceptions are BUSY and WEL. BUSY is hardware set and software
        /// cleared. WEL is hardware set and cleared by either hardware or
        /// software. These two bits are NOT reflected as part of a read to
        /// SPS_EEPROM_STATUS, please see SPS_EEPROM_BUSY_STATUS and
        /// SPS_EEPROM_WEL_STATUS. Also please reference the programmers model
        /// and spec for more details.
        (0x042c => eeprom_status: ReadWrite<u8>),
        (0x042d => _reserved042d),
        /// EEPROM busy status register. This bit is separated from the main
        /// generic status reg because it is set by hardware and cleared by
        /// software. The location of this bit is also programmable. Please
        /// reference the programmers model. To clear, write to the register
        (0x0430 => eeprom_busy_status: ReadWrite<u32, STATUS_BIT::Register>),
        /// One hot 8b register. the location of the 1 indicates where BUSY bit
        /// should be located. Note, BUSY cannot be at bit1 as that location is
        /// always reserved for WEL. Please see programmers model for more
        /// details. By default, the BUSY status is assumed to be at bit0
        (0x0434 => eeprom_busy_bit_vector: ReadWrite<u8>),
        (0x0435 => _reserved0435),
        /// EEPROM WEL status register. This bit is separated from the main
        /// generic status reg because it is set by hardware and cleared by
        /// hardware or software. This bit is always at position bit 1 of the
        /// status register. Please reference the programmers model. To clear,
        /// write to the register
        (0x0438 => eeprom_wel_status: ReadWrite<u32, STATUS_BIT::Register>),


        /// JEDEC ID value
        (0x043c => jedec_id: [ReadWrite<u32>; 3]),
        /// SFDP (Self discoverable parameter) 0..31
        (0x0448 => sfdp: [ReadWrite<u32>; 32]),


        /// Error return value to external host if SPI virtual address does
        /// not map to any component
        (0x04c8 => unmapped_return_val: ReadWrite<u8>),
        (0x04c9 => _reserved04c9),


        /// Virtual page mapping for page0..3 of on-die ram
        (0x04cc => ram_virtual_page: [ReadWrite<u32, PAGE::Register>; 4]),
        /// Controls specific to on-die ram mapping (bunker mode) during EEPROM
        /// operation
        (0x04dc => ram_ctrl_page: [ReadWrite<u32, RAM_CTRL_PAGE::Register>; 4]),
        /// Virtual base page of on-die flash
        (0x04ec => int_flash_base_page: ReadWrite<u32, PAGE::Register>),
        /// Virtual limit page of on-die flash (last page to be mapped)
        (0x04f0 => int_flash_limit_page: ReadWrite<u32, PAGE::Register>),
        /// Virtual base page of on-die flash
        (0x04f4 => ext_flash_base_page: ReadWrite<u32, PAGE::Register>),
        /// Virtual limit page of on-die flash (last page to be mapped)
        (0x04f8 => ext_flash_limit_page: ReadWrite<u32, PAGE::Register>),
        /// Bit vector of which bits of the address are translated. Bits to be
        /// translated are marked 0, bits not translated are marked 1.
        (0x04fc => int_flash_trans_bit_vector: ReadWrite<u32>),
        /// Address value to force in conjunction with the bit vector
        (0x0500 => int_flash_trans_addr: ReadWrite<u32>),
        /// Bit vector of which bits of the address are translated. Bits to be
        /// translated are marked 0, bits not translated are marked 1.
        (0x0504 => ext_flash_trans_bit_vector: ReadWrite<u32>),
        /// Address value to force in conjunction with the bit vector
        (0x0508 => ext_flash_trans_addr: ReadWrite<u32>),


        /// The current byte read pointer of the command memory, maintained by
        /// software. The MSB is used to detect if the memory is empty or full.
        (0x050c => cmd_mem_rptr: ReadWrite<u32, CMD_MEM_PTR::Register>),
        /// Top most value of the command address FIFO. Read this register
        /// generates a pulse that increments the FIFO read pointer
        (0x0510 => cmd_addr_fifo: ReadOnly<u32, CMD_MEM_PTR::Register>),
        /// command address fifo empty
        (0x0514 => cmd_addr_fifo_empty: ReadOnly<u32, STATUS_BIT::Register>),


        /// The base address of where the MSB rotated data structure is stored
        (0x0518 => fda_msb_rotate_base_addr: ReadWrite<u32>),
        /// The base address of where the MSB-1:MSB-2 rotated data structure is
        /// stored
        (0x051c => fda_msb_level2_rotate_base_addr: ReadWrite<u32>),


        /// Filter rules
        (0x0520 => passthru_filter_rule: [ReadWrite<u32, PASSTHRU_FILTER_RULE::Register>; 16]),
        /// Virtual address filter prior to mapping detection. This register is
        /// constructed as a bitmap, and directly corresponds to which bits to
        /// ignore. A value of 0 at bit31, means bit31 of the actual virtual
        /// address will be zerod out. For 3B mode, the value in the upper byte
        /// does not matter, as the controller will automatically pick the
        /// correct bits
        (0x0560 => virtual_addr_filter: ReadWrite<u32>),


        /// Debug register that tracks how many rising edges of CSB has been
        /// seen. Any write to this register sets the value back to 0
        (0x0564 => debug_cs_cnt: ReadWrite<u32>),
        (0x0568 => _reserved0568),


        /// Interrupt SPS_EEPROM_INT enable fields
        (0x0578 => eeprom_int_enable: ReadWrite<u32, EEPROM_INTERRUPT::Register>),
        /// Interrupt SPS_EEPROM_INT state fields
        (0x057c => eeprom_int_state: WriteOnly<u32, EEPROM_INTERRUPT::Register>),
        /// CPU interrupt SPS_EEPROM_INT test fields
        (0x0580 => eeprom_int_test: ReadWrite<u32, EEPROM_INTERRUPT::Register>),
        (0x0584 => _reserved0584),


        /// Generic RAM shared by both legacy and eeprom controllers
        (0x1000 => generic_ram: [ReadWrite<u8>; 2048]),
        (0x1800 => _reserved1800),
        /// Command buffer size (EEPROM mode)
        (0x2000 => eeprom_cmd_buf: [ReadWrite<u8>; 512]),
        (0x2200 => @END),
    }
}

register_bitfields![u32,
    CTRL [
        /// Mode
        MODE OFFSET(0) NUMBITS(2) [
            Generic = 0,
            SwetlandMode = 1,
            Eeprom = 2,
            Disabled = 3
        ],
        /// (Generic Mode) 0: Valid on first clock edge; 1: valid on second
        /// clock edge
        CPHA OFFSET(2) NUMBITS(1) [],
        /// (Generic Mode) 0: SCK start low; 1: SCK starts high
        CPOL OFFSET(3) NUMBITS(1) [],
        /// The polarity of the MISO pin during idle periods (CSB is
        /// deasserted).
        IDLE_LVL OFFSET(4) NUMBITS(1) [],
        /// (Generic Mode) 0: LSB sent first; 1: MSB send first
        TXBITOR OFFSET(5) NUMBITS(1) [],
        /// (Generic Mode) 0: LSB received first; 1: MSB received first
        RXBITOR OFFSET(6) NUMBITS(1) []
    ],
    FIFO_CTRL [
        /// Reset TX FIFO. Bit is self-clearing after write of 1.
        TXFIFO_RST OFFSET(0) NUMBITS(1) [],
        /// Enable transmission from TX FIFO
        TXFIFO_EN OFFSET(1) NUMBITS(1) [],
        /// Disable TX FIFO at the end of transaction
        TXFIFO_AUTO_DIS OFFSET(2) NUMBITS(1) [],
        /// Reset RX FIFO. Bit is self-clearing after write of 1.
        RXFIFO_RST OFFSET(3) NUMBITS(1) [],
        /// Enable packet receiving in RX FIFO
        RXFIFO_EN OFFSET(4) NUMBITS(1) [],
        /// Disable RX FIFO at the end of transaction
        RXFIFO_AUTO_DIS OFFSET(5) NUMBITS(1) []
    ],
    TXFIFO_SIZE [
        /// The number of bytes in the TX FIFO.
        VALUE OFFSET(0) NUMBITS(11) []
    ],
    TXFIFO_RPTR [
        /// The current byte read pointer of the TX FIFO. The MSB is used to
        /// detect if the TX FIFO is empty or full.
        VALUE OFFSET(0) NUMBITS(11) []
    ],
    TXFIFO_WPTR [
        /// The current byte write pointer of the TX FIFO. The MSB is used to
        /// detect if the TX FIFO is empty or full.
        VALUE OFFSET(0) NUMBITS(11) []
    ],
    TXFIFO_THRESHOLD [
        /// TXFIFO_LVL interrupt will be asserted when TXFIFO_SIZE is less than
        /// or equal to this value.
        VALUE OFFSET(0) NUMBITS(10) []
    ],
    RXFIFO_SIZE [
        /// The number of 8-bit words in the RX FIFO.
        VALUE OFFSET(0) NUMBITS(11) []
    ],
    RXFIFO_RPTR [
        /// The current byte read pointer of the RX FIFO. The MSB is used to
        /// detect if the RX FIFO is empty or full.
        VALUE OFFSET(0) NUMBITS(11) []
    ],
    RXFIFO_WPTR [
        /// The current byte write pointer of the RX FIFO. The MSB is used to
        /// detect if the RX FIFO is empty or full.
        VALUE OFFSET(0) NUMBITS(11) []
    ],
    RXFIFO_THRESHOLD [
        /// Level of RX FIFO + 1 in 8-bit words for RXFIFO_LVL interrupt
        VALUE OFFSET(0) NUMBITS(10) []
    ],
    INTERRUPT [
        /// Control Reg 0 interrupt
        CTLWR0 OFFSET(0) NUMBITS(1) [],
        /// Control Reg 1 interrupt
        CTLWR1 OFFSET(1) NUMBITS(1) [],
        /// Control Reg 2 interrupt
        CTLWR2 OFFSET(2) NUMBITS(1) [],
        /// Control Reg 3 interrupt
        CTLWR3 OFFSET(3) NUMBITS(1) [],
        /// Control Reg 4 interrupt
        CTLWR4 OFFSET(4) NUMBITS(1) [],
        /// Control Reg 5 interrupt
        CTLWR5 OFFSET(5) NUMBITS(1) [],
        /// Control Reg 6 interrupt
        CTLWR6 OFFSET(6) NUMBITS(1) [],
        /// Control Reg 7 interrupt
        CTLWR7 OFFSET(7) NUMBITS(1) [],
        /// CS assert interrupt
        CS_ASSERT OFFSET(8) NUMBITS(1) [],
        /// CS deassert interrupt
        CS_DEASSERT OFFSET(9) NUMBITS(1) [],
        /// RX FIFO overflow interrupt
        RXFIFO_OVERFLOW OFFSET(10) NUMBITS(1) [],
        /// TX FIFO empty interrupt
        TXFIFO_EMPTY OFFSET(11) NUMBITS(1) [],
        /// TX FIFO full interrupt
        TXFIFO_FULL OFFSET(12) NUMBITS(1) [],
        /// TX FIFO level interrupt
        TXFIFO_LVL OFFSET(13) NUMBITS(1) [],
        /// RX FIFO level interrupt
        RXFIFO_LVL OFFSET(14) NUMBITS(1) []
    ],
    ISTATE_CLR [
        /// Control Reg 0 interrupt clear
        CTLWR0 OFFSET(0) NUMBITS(1) [],
        /// Control Reg 1 interrupt clear
        CTLWR1 OFFSET(1) NUMBITS(1) [],
        /// Control Reg 2 interrupt clear
        CTLWR2 OFFSET(2) NUMBITS(1) [],
        /// Control Reg 3 interrupt clear
        CTLWR3 OFFSET(3) NUMBITS(1) [],
        /// Control Reg 4 interrupt clear
        CTLWR4 OFFSET(4) NUMBITS(1) [],
        /// Control Reg 5 interrupt clear
        CTLWR5 OFFSET(5) NUMBITS(1) [],
        /// Control Reg 6 interrupt clear
        CTLWR6 OFFSET(6) NUMBITS(1) [],
        /// Control Reg 7 interrupt clear
        CTLWR7 OFFSET(7) NUMBITS(1) [],
        /// CS assert interrupt clear
        CS_ASSERT OFFSET(8) NUMBITS(1) [],
        /// CS deassert interrupt clear
        CS_DEASSERT OFFSET(9) NUMBITS(1) [],
        /// RX FIFO overflow interrupt clear
        RXFIFO_OVERFLOW OFFSET(10) NUMBITS(1) []
    ],
    EEPROM_CTRL [
        /// SPI device EEPROM mode address selection. 0 -> 3 byte address for
        /// read commands. 1 -> 4 byte address for read commands
        ADDR_MODE OFFSET(0) NUMBITS(1) [],
        /// Disable passthrough filtering completely
        PASSTHRU_DIS OFFSET(1) NUMBITS(1) [],
        /// Disable external flash mapping completely
        EXT_FLASH_DIS OFFSET(2) NUMBITS(1) [],
        /// Disable internal flash mapping completely
        INT_FLASH_DIS OFFSET(3) NUMBITS(1) [],
        /// Disable internal ram mapping completely
        RAM_DIS OFFSET(4) NUMBITS(1) [],
        /// On-die ram is enabled for generic mailbox mode. Note when this field
        /// is 1, internal ram mapping is automatically disabled
        MAILBOX_EN OFFSET(5) NUMBITS(1) [],
        /// Prefetch limit for internal flash reads. This parameter controls how
        /// much controller prefetches into flash territory on reads mapped to
        /// internal flash. The default of 2 should be sufficient for most use
        /// cases. Setting this number too high can cause errors on transaction
        /// boundaries, as the flash subsystem can buffer up a large number of
        /// transactions that cross SPI transaction boundaries (across CSB)
        FIFO_PREFETCH_LIMIT OFFSET(6) NUMBITS(4) [],
        /// Dual read enable. 0 means dual read opcode is not treated as dual
        /// read
        FAST_DUAL_RD_EN OFFSET(10) NUMBITS(1) [],
        /// Enable virtual address filtering before use. This means address bits
        /// can be filtered out prior to region map. See SPS_VIRTUAL_ADDR_FILTER
        /// for more details
        VIRTUAL_ADDR_FILTER_EN OFFSET(11) NUMBITS(1) []
    ],
    BUSY_OPCODE [
        /// Arbitrary opcode is enabled
        EN OFFSET(0) NUMBITS(1) [],
        /// Value of arbitrary opcode
        VALUE OFFSET(1) NUMBITS(8) []
    ],
    STATUS_BIT [
        /// A single status bit
        VALUE OFFSET(0) NUMBITS(1) []
    ],
    PAGE [
        /// Page number (address shifted by PAGE_SIZE)
        ID OFFSET(0) NUMBITS(23) []
    ],
    RAM_CTRL_PAGE [
        /// When the end of a particular region is reached, the address
        /// automatically wraps to the beginning of the region
        WRAP_MODE OFFSET(0) NUMBITS(1) [],
        /// Watermark (in bytes) that triggers an interrupt to software
        INT_LVL OFFSET(1) NUMBITS(8) []
    ],
    CMD_MEM_PTR [
        /// Pointer into command memory
        VALUE OFFSET(0) NUMBITS(9) [],
        /// Indicate whether the memory is full
        FULL OFFSET(9) NUMBITS(1) []
    ],
    PASSTHRU_FILTER_RULE [
        /// Rule Valid.
        /// - Whether this rule participates in passthrough filtering.
        VALID OFFSET(0) NUMBITS(1) [],
        /// Reserved
        RESERVED OFFSET(1) NUMBITS(7) [],
        /// The command to force if rule is matched during filtering
        FORCE_CMD OFFSET(8) NUMBITS(8) [],
        /// The command value to match.
        CMD_MATCH OFFSET(16) NUMBITS(8) [],
        /// Command match bit vector
        /// - A bit vector to indicate how many bits should be compared.
        /// - This field helps differentiate the case between leading 0's and
        /// don't cares.
        CMD_MATCH_BIT_VECTOR OFFSET(24) NUMBITS(8) []
    ],
    EEPROM_INTERRUPT [
        /// INTR_CMD_ADDR_FIFO_NOT_EMPTY interrupt
        CMD_ADDR_FIFO_NOT_EMPTY OFFSET(0) NUMBITS(1) [],
        /// INTR_CMD_ADDR_FIFO_OVFL interrupt
        CMD_ADDR_FIFO_OVFL OFFSET(1) NUMBITS(1) [],
        /// INTR_CMD_MEM_OVFL interrupt
        CMD_MEM_OVFL OFFSET(2) NUMBITS(1) [],
        /// INTR_RAM_PAGE0_LVL interrupt
        RAM_PAGE0_LVL OFFSET(3) NUMBITS(1) [],
        /// INTR_RAM_PAGE1_LVL interrupt
        RAM_PAGE1_LVL OFFSET(4) NUMBITS(1) [],
        /// INTR_RAM_PAGE2_LVL interrupt
        RAM_PAGE2_LVL OFFSET(5) NUMBITS(1) [],
        /// INTR_RAM_PAGE3_LVL interrupt
        RAM_PAGE3_LVL OFFSET(6) NUMBITS(1) []
    ]
];

/// SPI device EEPROM virtual pages are 512 bytes in size.
const PAGE_SHIFT: u8 = 9;

#[allow(dead_code)]
const PAGE_SIZE: u32 = 1 << PAGE_SHIFT;

/// Configuration for SPI device hardware.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SpiDeviceConfiguration {
    /// Set to true to enable OpCode::FastRead4B.
    /// When set to false, enables OpCode::FastReadDualOutput.
    pub enable_fastread4b_cmd: bool,

    /// Set to true to handle OpCode::Enter4ByteAddressMode and OpCode::Exit4ByteAddressMode
    /// in software.
    /// When set to false, these op codes are not passed to software for handling.
    pub enable_enterexit4b_cmd: bool,

    /// Startup address mode.
    pub startup_address_mode: AddressMode,
}

impl SpiDeviceConfiguration {
    pub const fn default() -> SpiDeviceConfiguration {
        SpiDeviceConfiguration {
            enable_fastread4b_cmd: false,
            enable_enterexit4b_cmd: false,
            startup_address_mode: AddressMode::ThreeByte,
        }
    }
}

/// SPI device EEPROM sector size is 4KiB, since this is the smallest erasable
/// size.
#[allow(dead_code)]
const SECTOR_SIZE: u16 = 4096;

const SPI_DEVICE0_BASE_ADDR: u32 = 0x4051_0000;
const SPI_DEVICE0_REGISTERS: StaticRef<Registers> =
    unsafe { StaticRef::new(SPI_DEVICE0_BASE_ADDR as *const Registers) };

pub static mut SPI_DEVICE0: SpiDeviceHardware = SpiDeviceHardware::new(SPI_DEVICE0_REGISTERS,
    SpiDeviceConfiguration::default());

/// A SPI device
pub struct SpiDeviceHardware {
    registers: StaticRef<Registers>,
    client: OptionalCell<&'static dyn SpiDeviceClient>,
    config: SpiDeviceConfiguration,
}

impl SpiDeviceHardware {
    const fn new(base_addr: StaticRef<Registers>, config: SpiDeviceConfiguration) -> SpiDeviceHardware {
        SpiDeviceHardware {
            registers: base_addr,
            client: OptionalCell::empty(),
            config: config,
        }
    }

    pub fn init(&mut self, config: SpiDeviceConfiguration) {
        // First, disable everything
        self.registers.eeprom_int_enable.set(0);
        self.registers.ctrl.write(CTRL::MODE::Disabled);
        self.registers.eeprom_ctrl.write(
                EEPROM_CTRL::ADDR_MODE::CLEAR +
                EEPROM_CTRL::PASSTHRU_DIS::SET +
                EEPROM_CTRL::EXT_FLASH_DIS::SET +
                EEPROM_CTRL::INT_FLASH_DIS::SET +
                EEPROM_CTRL::RAM_DIS::SET +
                EEPROM_CTRL::MAILBOX_EN::CLEAR +
                EEPROM_CTRL::FIFO_PREFETCH_LIMIT.val(2) +
                EEPROM_CTRL::FAST_DUAL_RD_EN::SET +
                EEPROM_CTRL::VIRTUAL_ADDR_FILTER_EN::CLEAR
            );

        // Then, configure and enable features
        self.config = config;

        if self.config.enable_fastread4b_cmd {
            self.registers.fast_dual_rd_opcode.set(OpCode::FastRead4B as u8);
        } else {
            self.registers.fast_dual_rd_opcode.set(OpCode::FastReadDualOutput as u8);
        }

        self.init_passthrough_filters();

        self.init_jedec();

        self.init_busy_opcodes();

        self.set_address_mode(self.config.startup_address_mode);

        // Enable EEPROM mode
        self.registers.ctrl.modify(CTRL::MODE::Eeprom);
        self.enable_rx_interrupt();
    }

    fn init_jedec(&self) {
        // Set JEDEC ID to (incorrect) OpenTitan w/ 64 MiB visible flash space.
        self.registers.jedec_id[0].set(0x0020_3126);
        for idx in 1..self.registers.jedec_id.len() {
            self.registers.jedec_id[idx].set(0xffff_ffff);
        }
    }

    fn init_passthrough_filters(&self) {
        // Match 0b0000_0XXX (0x00 - 0x07) and force to NormalRead
        let mut rule_idx = 0;
        self.registers.passthru_filter_rule[rule_idx].write(
                PASSTHRU_FILTER_RULE::VALID::SET +
                PASSTHRU_FILTER_RULE::FORCE_CMD.val(OpCode::NormalRead as u32) +
                PASSTHRU_FILTER_RULE::CMD_MATCH.val(0b0000_0000) +
                PASSTHRU_FILTER_RULE::CMD_MATCH_BIT_VECTOR.val(0b1111_1000)
            );
        rule_idx += 1;

        if !self.config.enable_fastread4b_cmd {
            // Match 0b0000_1XXX (0x08 - 0x0f) and force to FastRead
            self.registers.passthru_filter_rule[rule_idx].write(
                    PASSTHRU_FILTER_RULE::VALID::SET +
                    PASSTHRU_FILTER_RULE::FORCE_CMD.val(OpCode::FastRead as u32) +
                    PASSTHRU_FILTER_RULE::CMD_MATCH.val(0b0000_1000) +
                    PASSTHRU_FILTER_RULE::CMD_MATCH_BIT_VECTOR.val(0b1111_1000)
                );
            rule_idx += 1;
        } else {
            // Match 0b0000_10XX (0x08 - 0x0b) and force to FastRead
            self.registers.passthru_filter_rule[rule_idx].write(
                    PASSTHRU_FILTER_RULE::VALID::SET +
                    PASSTHRU_FILTER_RULE::FORCE_CMD.val(OpCode::FastRead as u32) +
                    PASSTHRU_FILTER_RULE::CMD_MATCH.val(0b0000_1000) +
                    PASSTHRU_FILTER_RULE::CMD_MATCH_BIT_VECTOR.val(0b1111_1100)
                );
            rule_idx += 1;

            // Match 0b0000_11XX (0x0c - 0x0f) and force to FastRead4B
            self.registers.passthru_filter_rule[rule_idx].write(
                    PASSTHRU_FILTER_RULE::VALID::SET +
                    PASSTHRU_FILTER_RULE::FORCE_CMD.val(OpCode::FastRead4B as u32) +
                    PASSTHRU_FILTER_RULE::CMD_MATCH.val(0b0000_1100) +
                    PASSTHRU_FILTER_RULE::CMD_MATCH_BIT_VECTOR.val(0b1111_1100)
                );
            rule_idx += 1;
        }

        // Match 0b0001_XXXX (0x10 - 0x1f) and force to NormalRead
        self.registers.passthru_filter_rule[rule_idx].write(
                PASSTHRU_FILTER_RULE::VALID::SET +
                PASSTHRU_FILTER_RULE::FORCE_CMD.val(OpCode::NormalRead as u32) +
                PASSTHRU_FILTER_RULE::CMD_MATCH.val(0b0001_0000) +
                PASSTHRU_FILTER_RULE::CMD_MATCH_BIT_VECTOR.val(0b1111_0000)
            );
        rule_idx += 1;

        // Match 0b001X_XXXX (0x20 - 0x3f) and force to FastReadDualOutput
        self.registers.passthru_filter_rule[rule_idx].write(
                PASSTHRU_FILTER_RULE::VALID::SET +
                PASSTHRU_FILTER_RULE::FORCE_CMD.val(OpCode::FastReadDualOutput as u32) +
                PASSTHRU_FILTER_RULE::CMD_MATCH.val(0b0010_0000) +
                PASSTHRU_FILTER_RULE::CMD_MATCH_BIT_VECTOR.val(0b1110_0000)
            );
        rule_idx += 1;

        // Match 0b01XX_XXXX (0x40 - 0x7f) and force to NormalRead
        self.registers.passthru_filter_rule[rule_idx].write(
                PASSTHRU_FILTER_RULE::VALID::SET +
                PASSTHRU_FILTER_RULE::FORCE_CMD.val(OpCode::NormalRead as u32) +
                PASSTHRU_FILTER_RULE::CMD_MATCH.val(0b0100_0000) +
                PASSTHRU_FILTER_RULE::CMD_MATCH_BIT_VECTOR.val(0b1100_0000)
            );
        rule_idx += 1;

        // Match 0b1XXX_XXXX (0x80 - 0xff) and force to NormalRead
        self.registers.passthru_filter_rule[rule_idx].write(
                PASSTHRU_FILTER_RULE::VALID::SET +
                PASSTHRU_FILTER_RULE::FORCE_CMD.val(OpCode::NormalRead as u32) +
                PASSTHRU_FILTER_RULE::CMD_MATCH.val(0b1000_0000) +
                PASSTHRU_FILTER_RULE::CMD_MATCH_BIT_VECTOR.val(0b1000_0000)
            );
        rule_idx += 1;

        // Disable all remaining passthrough filter rules
        for idx in rule_idx..self.registers.passthru_filter_rule.len() {
            self.registers.passthru_filter_rule[idx].write(PASSTHRU_FILTER_RULE::VALID::CLEAR);
        }

        self.registers.eeprom_ctrl.modify(EEPROM_CTRL::PASSTHRU_DIS::CLEAR);
    }

    fn init_busy_opcodes(&self) {
        let mut opcode_idx = 0;
        self.registers.busy_opcode[opcode_idx].write(
            BUSY_OPCODE::EN::SET +
            BUSY_OPCODE::VALUE.val(OpCode::Enter4ByteAddressMode as u32)
        );
        opcode_idx += 1;

        self.registers.busy_opcode[opcode_idx].write(
            BUSY_OPCODE::EN::SET +
            BUSY_OPCODE::VALUE.val(OpCode::Exit4ByteAddressMode as u32)
        );
        opcode_idx += 1;

        for idx in opcode_idx..self.registers.busy_opcode.len() {
            self.registers.busy_opcode[idx].write(BUSY_OPCODE::EN::CLEAR);
        }
    }

    fn clear_rx_interrupt(&self) {
        self.registers.eeprom_int_state.write(EEPROM_INTERRUPT::CMD_ADDR_FIFO_NOT_EMPTY::SET);
    }

    fn enable_rx_interrupt(&self) {
        self.registers.eeprom_int_enable.modify(EEPROM_INTERRUPT::CMD_ADDR_FIFO_NOT_EMPTY::SET);
    }

    #[allow(dead_code)]
    fn disable_rx_interrupt(&self) {
        self.registers.eeprom_int_enable.modify(EEPROM_INTERRUPT::CMD_ADDR_FIFO_NOT_EMPTY::CLEAR);
    }

    fn is_busy(&self) -> bool {
        let busy = self.registers.eeprom_busy_status.is_set(STATUS_BIT::VALUE);
        busy
    }

    pub fn handle_interrupt_cmd_addr_fifo_not_empty(&self) {
        //debug!("CMD_ADDR_FIFO_EMPTY = {}", self.registers.cmd_addr_fifo_empty.get());
        if !self.registers.cmd_addr_fifo_empty.is_set(STATUS_BIT::VALUE) {
            self.client.map(|client| {
                client.data_available(self.is_busy());
            });
        }

        self.clear_rx_interrupt();
    }
}

impl SpiDevice for SpiDeviceHardware {
    fn set_client(&self, client: Option<&'static dyn SpiDeviceClient>) {
        //debug!("kernel: set_client: client={}", if client.is_some() { "Some" } else { "None" });
        match client {
            None => { self.client.clear(); }
            Some(cl) => { self.client.set(cl); }
        }
    }

    fn configure_addresses(&self, config: AddressConfig) {
        self.registers.eeprom_ctrl.modify(EEPROM_CTRL::EXT_FLASH_DIS::SET);
        self.registers.eeprom_ctrl.modify(EEPROM_CTRL::VIRTUAL_ADDR_FILTER_EN::CLEAR);
        self.registers.eeprom_ctrl.modify(EEPROM_CTRL::RAM_DIS::SET);

        // Configure external flash at `flash_virtual_base` with length `size`
        self.registers.ext_flash_base_page.write(
            PAGE::ID.val(config.flash_virtual_base >> PAGE_SHIFT));
        self.registers.ext_flash_limit_page.write(
            PAGE::ID.val((config.flash_virtual_base + (config.flash_physical_size - 1)) >> PAGE_SHIFT));

        // Zero out all address bits beyond the size of the flash chip
        self.registers.ext_flash_trans_bit_vector.set(
            config.flash_virtual_base + (config.flash_physical_size - 1));

        // Configure mapping to `physical_base`.
        self.registers.ext_flash_trans_addr.set(
            config.flash_physical_base);


        // Configure all available EEPROM mode RAM pages after EXT_FLASH
        let ram_virtual_page_base = config.ram_virtual_base >> PAGE_SHIFT;
        for idx in 0..self.registers.ram_virtual_page.len() {
            self.registers.ram_virtual_page[idx].write(
                PAGE::ID.val(ram_virtual_page_base + idx as u32));
            self.registers.ram_ctrl_page[idx].write(
                RAM_CTRL_PAGE::WRAP_MODE::CLEAR +
                RAM_CTRL_PAGE::INT_LVL.val(0)
            );
        }


        // Only allow addresses within the virtual_size to allow space for
        // external flash and RAM pages
        self.registers.virtual_addr_filter.set(config.virtual_size - 1);


        self.registers.eeprom_ctrl.modify(EEPROM_CTRL::EXT_FLASH_DIS::CLEAR);
        self.registers.eeprom_ctrl.modify(EEPROM_CTRL::RAM_DIS::CLEAR);
        self.registers.eeprom_ctrl.modify(EEPROM_CTRL::VIRTUAL_ADDR_FILTER_EN::SET);
    }

    fn set_address_mode(&self, address_mode: AddressMode) {
        match address_mode {
            AddressMode::ThreeByte => self.registers.eeprom_ctrl.modify(EEPROM_CTRL::ADDR_MODE::CLEAR),
            AddressMode::FourByte => self.registers.eeprom_ctrl.modify(EEPROM_CTRL::ADDR_MODE::SET),
        }
    }

    fn get_address_mode(&self) -> AddressMode {
        if self.registers.eeprom_ctrl.is_set(EEPROM_CTRL::ADDR_MODE) {
            AddressMode::FourByte
        } else {
            AddressMode::ThreeByte
        }
    }

    fn get_received_data(&self, read_buffer: &mut[u8]) -> usize {
        if self.registers.cmd_addr_fifo_empty.is_set(STATUS_BIT::VALUE) {
            return 0;
        }

        // Copy cmd_addr_fifo register since reading it advances it.
        let cmd_addr_fifo_reg = self.registers.cmd_addr_fifo.extract();

        let start_addr = self.registers.cmd_mem_rptr.read(CMD_MEM_PTR::VALUE) as usize;
        let end_addr = cmd_addr_fifo_reg.read(CMD_MEM_PTR::VALUE) as usize;
        //debug!("start={:08x} end={:08x}", start_addr, end_addr);
        //debug!("fifo_full={} rptr_full={}",
        //    cmd_addr_fifo_reg.read(CMD_MEM_PTR::FULL),
        //    self.registers.cmd_mem_rptr.read(CMD_MEM_PTR::FULL));
        let mut length : usize = 0;

        if start_addr < end_addr {
            // Read data bytes from start_addr to end_addr-1
            length = min(read_buffer.len(), end_addr-start_addr);
            let mut tgt_idx : usize = 0;
            for idx in start_addr..end_addr {
                if tgt_idx >= length { break; }
                read_buffer[tgt_idx] = self.registers.eeprom_cmd_buf[idx].get();
                tgt_idx += 1;
            }
        } else if cmd_addr_fifo_reg.read(CMD_MEM_PTR::FULL) !=
            self.registers.cmd_mem_rptr.read(CMD_MEM_PTR::FULL) {
            // Read data bytes from start_addr to cmd_buf.len.
            // Then append data from 0 to end_addr-1.
            length = min(read_buffer.len(),
                self.registers.eeprom_cmd_buf.len() - start_addr + end_addr);
            let mut tgt_idx : usize = 0;
            for src_idx in start_addr..self.registers.eeprom_cmd_buf.len() {
                if tgt_idx >= length { break; }
                read_buffer[tgt_idx] = self.registers.eeprom_cmd_buf[src_idx].get();
                tgt_idx += 1;
            }
            for src_idx in 0..end_addr {
                if tgt_idx >= length { break; }
                read_buffer[tgt_idx] = self.registers.eeprom_cmd_buf[src_idx].get();
                tgt_idx += 1;
            }
        }
        debug!("length={}", length);

        // Update rptr since we now read all the data.
        self.registers.cmd_mem_rptr.set(cmd_addr_fifo_reg.get());

        // Return length of data
        length
    }

    fn put_send_data(&self, write_data: &[u8]) -> kernel::ReturnCode {
        debug!("kernel: put_send_data (len={})", write_data.len());
        if write_data.len() > self.registers.generic_ram.len() {
            debug!("h1::Sps::store_data: Invalid write_data length == {}", write_data.len());
            return ReturnCode::ESIZE;
        }
        for idx in 0..write_data.len() {
            self.registers.generic_ram[idx].set(write_data[idx]);
        }
        for idx in write_data.len()..self.registers.generic_ram.len() {
            self.registers.generic_ram[idx].set(0xff);
        }

        ReturnCode::SUCCESS
    }

    fn clear_busy(&self) {
        // Note that this setting will not take effect until the SPI host reads
        // out the status register
        self.registers.eeprom_busy_status.set(1);
    }
}
