//! Interfaces for SPI host on H1

pub trait SpiHost {

    /// Enable/disable SPI device <-> SPI host pass through
    fn spi_device_spi_host_passthrough(&self, enable: bool);
}
