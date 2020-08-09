//! Interfaces for SPI host on H1

pub trait SpiHost {
    /// Enable/disable SPI device <-> SPI host pass through
    ///
    /// `enable`: Whether to enable (`true`) or disable (`false`) the pass
    /// through.
    fn spi_device_spi_host_passthrough(&self, enable: bool);

    /// Enable/disable waiting for BUSY bit in status register of connected SPI
    /// flash device to clear at the end of transactions by polling the status
    /// register.
    ///
    /// If enabled, transactions are marked completed only when the BUSY bit
    /// has been cleared. If disabled the the status register is not polled and
    /// transaction are completed immediately.
    ///
    /// `enable`: Whether to enable (`true`) or disable (`false`) waiting for
    /// the BUSY bit to be cleared.
    fn wait_busy_clear_in_transactions(&self, enable: bool);
}
