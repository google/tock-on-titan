use super::serialize::Serialize;

#[repr(C)]
pub struct DeviceDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub bcd_usb: u16,
    pub b_device_class: u8,
    pub b_device_sub_class: u8,
    pub b_device_protocol: u8,
    pub b_max_packet_size0: u8,
    pub id_vendor: u16,
    pub id_product: u16,
    pub bcd_device: u16,
    pub i_manufacturer: u8,
    pub i_product: u8,
    pub i_serial_number: u8,
    pub b_num_configurations: u8,
}

unsafe impl Serialize for DeviceDescriptor {}

pub struct ConfigurationDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub w_total_length: u16,
    pub b_num_interfaces: u8,
    pub b_configuration_value: u8,
    pub i_configuration: u8,
    pub bm_attributes: u8,
    pub b_max_power: u8,
}

impl ConfigurationDescriptor {
    /// Creates an empty configuration with no interface descriptors.
    ///
    /// `bm_attributes` set to self powered, and not remote wakeup
    /// `b_max_power` set to 100ma
    pub fn new() -> ConfigurationDescriptor {
        ConfigurationDescriptor {
            b_length: 9,
            b_descriptor_type: 2,
            w_total_length: 9,
            b_num_interfaces: 0,
            b_configuration_value: 1,
            i_configuration: 0,
            bm_attributes: 0b11000000,
            b_max_power: 50,
        }
    }
}

unsafe impl Serialize for ConfigurationDescriptor {}

#[repr(u8)]
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum SetupRequestType {
    GetStatus = 0,
    ClearFeature = 1,

    SetFeature = 3,

    SetAddress = 5,
    GetDescriptor = 6,
    SetDescriptor = 7,
    GetConfiguration = 8,
    SetConfiguration = 9,
    GetInterface = 10,
    SetInterface = 11,
    SynchFrame = 12,
}

pub struct SetupRequest {
    pub bm_request_type: u8,
    pub b_request: SetupRequestType,
    pub w_value: u16,
    pub w_index: u16,
    pub w_length: u16,
}

impl SetupRequest {
    pub fn parse(buf: &[u8; 64]) -> &SetupRequest {
        unsafe { ::core::mem::transmute(buf.as_ptr()) }
    }

    pub fn data_direction(&self) -> u8 {
        (self.bm_request_type & 0x80) >> 7
    }

    pub fn req_type(&self) -> u8 {
        (self.bm_request_type & 0x60) >> 5
    }

    pub fn recipient(&self) -> u8 {
        self.bm_request_type & 0x1f
    }
}
