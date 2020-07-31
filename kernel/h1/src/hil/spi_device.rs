//! Interfaces for SPI device on H1

use spiutils::protocol::flash::AddressMode;

/// Address configuration for SPI device hardware.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AddressConfig {
    /// The address on the SPI device bus that the external flash is accessible at.
    pub flash_virtual_base: u32,

    /// The base address in the external flash device on the SPI host bus.
    pub flash_physical_base: u32,

    /// The size of the external flash device.
    /// This must be a 2^N.
    pub flash_physical_size: u32,

    /// The address on the SPI device bus that the RAM (mailbox) is accessible at.
    pub ram_virtual_base: u32,

    /// The total size available on the SPI device bus.
    /// This must be a 2^N.
    pub virtual_size: u32,
}

pub trait SpiDeviceClient {
    /// Called when data from the SPI host is available.
    ///
    /// `is_busy`: Whether the command caused the "busy" bit to be set.
    /// If the "busy" bit has been set, then SpiDevice.clear_busy must
    /// be called to finish the transaction.
    fn data_available(&self, is_busy: bool);
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

    /// Clear the busy bit.
    fn clear_busy(&self);
}
