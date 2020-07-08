//! Interfaces for SPI device on H1

pub trait SpiDeviceClient {
    /// Called when data from the SPI host is available.
    /// is_busy: Whether the command caused the "busy" bit to be set.
    ///          If the "busy" bit has been set, then SpiDevice.clear_busy
    ///          must be called to finish the transaction.
    fn data_available(&self, is_busy: bool);
}

pub trait SpiDevice {

    fn set_client(&self, client: core::option::Option<&'static dyn SpiDeviceClient>);

    /// Put engine into 4-byte address mode.
    fn enter_4b_mode(&self);

    /// Put engine into 3-byte address mode.
    fn exit_4b_mode(&self);

    /// Get data received from the SPI host.
    fn get_received_data(&self, read_buffer: &mut[u8]) -> usize;

    /// Put data to send to the SPI host.
    fn put_send_data(&self, write_data: &[u8]) -> kernel::ReturnCode;

    /// Clear the busy bit.
    fn clear_busy(&self);
}