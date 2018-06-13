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

#[derive(Debug)]
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
    /// Take the configuration and write it out as bytes into
    /// the buffer, returning the number of bytes written.
    pub fn into_buf(&self, buf: &mut [u32; 64]) -> usize {
        buf[0] = (self.b_length as u32)          <<  0 |
                 (self.b_descriptor_type as u32) <<  8 |
                 (self.w_total_length as u32)    << 16;
        buf[1] = (self.b_num_interfaces as u32)      <<  0 |
                 (self.b_configuration_value as u32) <<  8 |
                 (self.i_configuration as u32)       << 16 |
                 (self.bm_attributes as u32)         << 24;
        buf[2] = (self.b_max_power as u32) << 0;
        9
    }
}

unsafe impl Serialize for ConfigurationDescriptor {}

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
#[repr(u8)]
pub enum SetupRequestType {
    GetStatus = 0,
    ClearFeature = 1,
    Reserved = 2,
    SetFeature = 3,

    SetAddress = 5,
    GetDescriptor = 6,
    SetDescriptor = 7,
    GetConfiguration = 8,
    SetConfiguration = 9,
    GetInterface = 10,
    SetInterface = 11,
    SynchFrame = 12,
    Undefined = 15,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub enum SetupDirection {
    HostToDevice = 0,
    DeviceToHost = 1,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub enum SetupRequestClass {
    Standard = 0,
    Class    = 1,
    Vendor   = 2,
    Reserved = 3,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub enum SetupRecipient {
    Device    = 0,
    Interface = 1,
    Endpoint  = 2,
    Other     = 3,
    Reserved  = 4,
}

pub struct SetupRequest {
    pub bm_request_type: u8,
    pub b_request: u8,
    pub w_value: u16,
    pub w_index: u16,
    pub w_length: u16,
}

impl SetupRequest {

    pub fn new(buf: &[u32; 16]) -> SetupRequest {
        SetupRequest {
            bm_request_type: (buf[0] & 0xff) as u8,
            b_request:      ((buf[0] & 0x0000ff00) >> 8) as u8,
            w_value:        ((buf[0] & 0xffff0000) >> 16) as u16,
            w_index:         (buf[1] & 0x0000ffff) as u16,
            w_length:       ((buf[1] & 0xffff0000) >> 16) as u16,
        }
    }
   
    pub fn parse(buf: &[u32; 16], req: &mut SetupRequest) {
        req.bm_request_type = (buf[0] & 0xff) as u8;
        req.b_request =      ((buf[0] & 0x0000ff00) >> 8) as u8;
        req.w_value =        ((buf[0] & 0xffff0000) >> 16) as u16;
        req.w_index =         (buf[1] & 0x0000ffff) as u16;
        req.w_length =       ((buf[1] & 0xffff0000) >> 16) as u16
    }

    // 0 is Host-to-Device, 1 is Device-to-Host
    pub fn data_direction(&self) -> SetupDirection {
        let val = (self.bm_request_type & 0x80) >> 7;
        match val {
            0 => SetupDirection::HostToDevice,
            _ => SetupDirection::DeviceToHost
        }
    }

    // 0 is Standard, 1 is Class, 2 is Vendor, 3 is Reserved
    pub fn req_type(&self) -> SetupRequestClass {
        let val = (self.bm_request_type & 0x60) >> 5;
        match val {
            0 => SetupRequestClass::Standard,
            1 => SetupRequestClass::Class,
            2 => SetupRequestClass::Vendor,
            _ => SetupRequestClass::Reserved,
        }
    }

    // 0 is Device, 1 is Interface, 2 is Endpoint, 3 is Other
    // 4..31 are Reserved
    pub fn recipient(&self) -> SetupRecipient {
        let val = self.bm_request_type & 0x1f;
        match val {
            0 => SetupRecipient::Device,
            1 => SetupRecipient::Interface,
            2 => SetupRecipient::Endpoint,
            3 => SetupRecipient::Other,
            _ => SetupRecipient::Reserved,
        }
    }

    pub fn request(&self) -> SetupRequestType {
        match self.b_request {
            0 => SetupRequestType::GetStatus,
            1 => SetupRequestType::ClearFeature,
            2 => SetupRequestType::Reserved,
            3 => SetupRequestType::SetFeature,
            4 => SetupRequestType::Reserved,
            5 => SetupRequestType::SetAddress,
            6 => SetupRequestType::GetDescriptor,
            7 => SetupRequestType::SetDescriptor,
            8 => SetupRequestType::GetConfiguration,
            9 => SetupRequestType::SetConfiguration,
            10 => SetupRequestType::GetInterface,
            11 => SetupRequestType::SetInterface,
            12 => SetupRequestType::SynchFrame,
             _ => SetupRequestType::Undefined
        }
    }
}
