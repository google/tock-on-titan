use crate::hil::spi_host::SpiHost;
use core::cell::Cell;
use core::cmp::min;
use kernel::common::cells::{OptionalCell, TakeCell};
use kernel::common::registers::{register_bitfields, register_structs, ReadOnly, ReadWrite, WriteOnly};
use kernel::common::StaticRef;
use kernel::hil::spi::{ClockPolarity, ClockPhase, SpiMaster, SpiMasterClient};
use kernel::ReturnCode;

// The TX and RX FIFOs both have the same length. We write and read at the same time.

// Registers for the SPI host controller
register_structs! {
    Registers {
        (0x0000 => ctrl: ReadWrite<u32, CTRL::Register>),
        (0x0004 => xact: ReadWrite<u32, XACT::Register>),
        (0x0008 => ictrl: ReadWrite<u32, ICTRL::Register>),
        (0x000c => istate: ReadOnly<u32, ISTATE::Register>),
        (0x0010 => istate_clr: ReadWrite<u32, ISTATE_CLR::Register>),
        (0x0014 => _reserved),
        (0x1000 => tx_fifo: [WriteOnly<u8>; 128]),
        (0x1080 => rx_fifo: [ReadOnly<u8>; 128]),
        (0x1100 => @END),
    }
}

register_bitfields![u32,
    CTRL [
        /// CPOL setting
        CPOL OFFSET(0) NUMBITS(1) [],
        /// CPHA setting
        CPHA OFFSET(1) NUMBITS(1) [],
        /// CSB to SCK setup time in SCK cycles + 1.5
        CSBSU OFFSET(2) NUMBITS(4) [],
        /// CSB from SCK hold time in SCK cycles + 1 (defined with respect to the last SCK edge)
        CSBHLD OFFSET(6) NUMBITS(4) [],
        /// SPI Clk Divider. Actual divider is IDIV+1. A value of 0 gives divide by 1 clock, 1 gives divide by 2 etc.
        IDIV OFFSET(10) NUMBITS(12) [],
        /// Polarity of CSB signal. 0:active low 1:active high
        CSBPOL OFFSET(22) NUMBITS(1) [],
        /// Order in which bits of byte are sent. 0: send bit 0 first. 1: send bit 7 first
        TXBITOR OFFSET(23) NUMBITS(1) [],
        /// Order in which bytes of buffer word are sent. 0: send byte 0 first. 1: send byte 3 first
        TXBYTOR OFFSET(24) NUMBITS(1) [],
        /// Order in which received bits are packed into byte. 0: first bit received is bit0 1: last bit received is bit 0
        RXBITOR OFFSET(25) NUMBITS(1) [],
        /// Order in which received bytes are packed into word. 0: first byte received is byte 0 1: first byte received is byte 3
        RXBYTOR OFFSET(26) NUMBITS(1) [],
        /// SPI Passthrough Mode. 0: Disable, 1: Enable. This is the host side control of whether passthrough is allowed. In order for full passthrough functionality, both the host and device passthrough functionality have to be enabled
        ENPASSTHRU OFFSET(27) NUMBITS(1) []
    ],
    XACT [
        /// Initiate transaction in buffer
        START OFFSET(0) NUMBITS(1) [],
        /// Bits-1 in last byte transferred. The default assumes last byte will have 8 bits, this should be sufficient for most usage.
        BCNT OFFSET(1) NUMBITS(3) [],
        /// Total number of transactions in bytes-1. If 64 bytes are to be transferred, this should be programmed as 63.
        SIZE OFFSET(4) NUMBITS(7) [],
        /// Poll for ready
        RDY_POLL OFFSET(11) NUMBITS(1) [],
        /// Delay before polling in PCLK cycles + 1
        RDY_POLL_DLY OFFSET(12) NUMBITS(5) []
    ],
    ICTRL [
        /// TX interrupt enable
        TXDONE OFFSET(0) NUMBITS(1) []
    ],
    ISTATE [
        /// TX done interrupt
        TXDONE OFFSET(0) NUMBITS(1) []
    ],
    ISTATE_CLR [
        /// TX done interrupt clear
        TXDONE OFFSET(0) NUMBITS(1) []
    ]
];

const SPI_HOST0_BASE_ADDR: u32 = 0x4070_0000;
const SPI_HOST1_BASE_ADDR: u32 = 0x4071_0000;

const SPI_HOST0_REGISTERS: StaticRef<Registers> =
    unsafe { StaticRef::new(SPI_HOST0_BASE_ADDR as *const Registers) };
const SPI_HOST1_REGISTERS: StaticRef<Registers> =
    unsafe { StaticRef::new(SPI_HOST1_BASE_ADDR as *const Registers) };

pub static mut SPI_HOST0: SpiHostHardware = SpiHostHardware::new(SPI_HOST0_REGISTERS);

pub static mut SPI_HOST1: SpiHostHardware = SpiHostHardware::new(SPI_HOST1_REGISTERS);

/// A SPI Host
pub struct SpiHostHardware {
    registers: StaticRef<Registers>,
    transaction_len: Cell<usize>,
    tx_buffer: TakeCell<'static, [u8]>,
    rx_buffer: TakeCell<'static, [u8]>,
    client: OptionalCell<&'static dyn SpiMasterClient>,
}

impl SpiHostHardware {
    const fn new(base_addr: StaticRef<Registers>) -> SpiHostHardware {
        SpiHostHardware {
            registers: base_addr,
            transaction_len: Cell::new(0),
            tx_buffer: TakeCell::empty(),
            rx_buffer: TakeCell::empty(),
            client: OptionalCell::empty(),
        }
    }

    pub fn init(&self) {
        self.registers.ctrl.write(
            CTRL::CPOL::CLEAR +
            CTRL::CPHA::CLEAR +
            CTRL::CSBSU::CLEAR +
            CTRL::CSBHLD::CLEAR +
            CTRL::IDIV.val(2) +
            CTRL::CSBPOL::CLEAR +
            CTRL::TXBITOR::SET +
            CTRL::TXBYTOR::CLEAR +
            CTRL::RXBITOR::SET +
            CTRL::RXBYTOR::CLEAR +
            CTRL::ENPASSTHRU::CLEAR);
    }

    fn enable_tx_interrupt(&self) {
        self.registers.ictrl.modify(ICTRL::TXDONE::SET);
    }

    fn disable_tx_interrupt(&self) {
        self.registers.ictrl.modify(ICTRL::TXDONE::CLEAR);
    }

    #[inline(never)]
    pub fn handle_interrupt(&self) {
        //debug!("h1::Spi:handle_interrupt: ISTATE = {:08x}", self.registers.istate.get());
        if self.registers.istate.is_set(ISTATE::TXDONE) {
            self.registers.istate_clr.write(ISTATE_CLR::TXDONE::SET);
            self.client.map(|client| {
                self.tx_buffer.take()
                .map(|tx_buf| {
                    self.rx_buffer
                    .map(|rx_buf| {
                        self.read_data(rx_buf);
                    });

                    client.read_write_done(tx_buf, self.rx_buffer.take(), self.transaction_len.get())
                });
            });
        }
        self.disable_tx_interrupt();
    }

    fn start_transaction(&self, write_buffer: Option<&'static mut [u8]>, read_buffer: Option<&'static mut [u8]>, transaction_len: usize) -> ReturnCode {
        //debug!("h1::Spi:start_transaction: transaction_len={}", transaction_len);
        // The transaction needs at least one byte.
        // It also cannot have more bytes than tx_fifo or rx_fifo is long.
        if (transaction_len == 0) || (transaction_len >= self.registers.tx_fifo.len()) || (transaction_len >= self.registers.rx_fifo.len()) {
            debug!("h1::Spi::start_transaction: Invalid transaction_len={}", transaction_len);
            return ReturnCode::ESIZE;
        }
        self.registers.xact.modify(XACT::BCNT.val(7));
        self.registers.xact.modify(XACT::SIZE.val((transaction_len - 1) as u32));

        let mut tx_buf_len = 0;
        write_buffer.as_ref().map(|tx_buf| {
            tx_buf_len = min(tx_buf.len(), transaction_len);
            for idx in 0..tx_buf_len {
                self.registers.tx_fifo[idx].set(tx_buf[idx]);
            }
        });

        // Clear the TX fifo for additional bytes not supplied by write_buffer
        for idx in tx_buf_len..transaction_len {
            self.registers.tx_fifo[idx].set(0xff);
        }

        write_buffer.map(|buf| {
            self.tx_buffer.replace(buf);
        });
        read_buffer.map(|buf| {
            self.rx_buffer.replace(buf);
        });
        self.transaction_len.set(transaction_len);

        self.registers.istate_clr.write(ISTATE_CLR::TXDONE::SET);
        self.enable_tx_interrupt();
        self.registers.xact.modify(XACT::START::SET);
        ReturnCode::SUCCESS
    }

    fn read_data(&self, read_buffer: &mut [u8]) {
        let read_len = min(read_buffer.len(), self.transaction_len.get());
        for idx in 0..read_len {
            let val = self.registers.rx_fifo[idx].get();
            read_buffer[idx] = val;
        }
    }
}

impl SpiHost for SpiHostHardware {
    fn spi_device_spi_host_passthrough(&self, enabled: bool) {
        self.registers.ctrl.modify(if enabled { CTRL::ENPASSTHRU::SET } else { CTRL::ENPASSTHRU::CLEAR });
    }
}

impl SpiMaster for SpiHostHardware {
    type ChipSelect = bool;

    fn set_client(&self, client: &'static dyn kernel::hil::spi::SpiMasterClient) {
        self.client.set(client);
    }

    fn init(&self) {

    }

    fn is_busy(&self) -> bool {
        self.registers.istate.is_set(ISTATE::TXDONE)
    }

    fn read_write_bytes(
        &self,
        write_buffer: &'static mut [u8],
        read_buffer: Option<&'static mut [u8]>,
        len: usize,
    ) -> ReturnCode {
        // If busy, don't start
        if self.is_busy() {
            return ReturnCode::EBUSY;
        }

        self.start_transaction(Some(write_buffer), read_buffer, len)
    }

    fn write_byte(&self, _val: u8) {
        panic!("write_byte is not implemented");
    }
    fn read_byte(&self) -> u8 {
        panic!("read_byte is not implemented");
    }
    fn read_write_byte(&self, _val: u8) -> u8 {
        panic!("read_write_byte is not implemented");
    }

    fn specify_chip_select(&self, _cs: Self::ChipSelect) {
        // Nothing to be done
    }

    /// Returns the actual rate set
    fn set_rate(&self, _rate: u32) -> u32 {
        panic!("set_rate is not implemented");
    }
    fn get_rate(&self) -> u32 {
        panic!("get_rate is not implemented");
    }
    fn set_clock(&self, _polarity: ClockPolarity) {
        panic!("set_clock is not implemented");
    }
    fn get_clock(&self) -> ClockPolarity {
        panic!("get_clock is not implemented");
    }
    fn set_phase(&self, _phase: ClockPhase) {
        panic!("set_phase is not implemented");
    }
    fn get_phase(&self) -> ClockPhase {
        panic!("get_phase is not implemented");
    }

    fn hold_low(&self) {
        panic!("hold_low is not implemented");
    }
    fn release_low(&self) {
        // Nothing to do, since this is the only mode supported.
    }
}
