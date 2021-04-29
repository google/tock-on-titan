//! Interfaces for SPI device on H1

use spiutils::driver::spi_device::AddressConfig;
use spiutils::protocol::flash::AddressMode;

pub trait SpiDeviceClient {
    /// Called when data from the SPI host is available.
    ///
    /// `is_busy`: Whether the command caused the "busy" bit to be set.
    /// If the "busy" bit has been set, then SpiDevice.clear_busy must
    /// be called to finish the transaction.
    ///
    /// `is_write_enabled`: Whether the "write enabled" bit is set.
    fn data_available(&self, is_busy: bool, is_write_enabled: bool);
}

pub trait SpiDevice {
    fn set_client(&self, client: Option<&'static dyn SpiDeviceClient>);

    /// Configure addresses exposed by this SPI device on the SPI bus.
    fn configure_addresses(&self, config: AddressConfig);

    /// Configure the engine's address mode.
    fn set_address_mode(&self, address_mode: AddressMode);

    /// Get the engine's address mode.
    fn get_address_mode(&self) -> AddressMode;

    /// Get data received from the SPI host.
    ///
    /// `read_buffer`: Received data is written into this buffer.
    ///
    /// Returns the length of data written into `read_buffer`.
    fn get_received_data(&self, read_buffer: &mut [u8]) -> usize;

    /// Put data to send to the SPI host.
    ///
    /// `write_data`: All data from this buffer is copied into the HW buffer.
    /// If the `write_data` buffer is shorter than the HW buffer, the HW buffer
    /// is padded with 0xFF.
    fn put_send_data(&self, write_data: &[u8]) -> kernel::ReturnCode;

    /// Set the contents of the SPI flash status register.
    /// Note that this does not include the busy bit and the write enable bit.
    fn set_status(&self, status: u8);

    /// Clear the busy bit.
    fn clear_busy(&self);

    /// Returns true if the write enable bit is set.
    fn is_write_enable_set(&self) -> bool;

    /// Clear the write enable bit.
    fn clear_write_enable(&self);

    /// Configure JEDEC ID
    fn set_jedec_id(&self, data: &[u8]) -> kernel::ReturnCode;

    /// Configure SFDP
    fn set_sfdp(&self, data: &[u8]) -> kernel::ReturnCode;
}
