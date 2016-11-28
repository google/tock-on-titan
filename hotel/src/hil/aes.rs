#[derive(Debug, Copy, Clone)]
pub enum KeySize {
    /// Uses 128 bit AES key
    KeySize128 = 0x0,
    /// Uses 192 bit AES key
    KeySize192 = 0x2,
    /// Uses 256 bit AES key
    KeySize256 = 0x4,
}

#[derive(Debug, Copy, Clone)]
pub enum CipherMode {
    /// Electronic Codebook mode.
    Ecb = 0x0,
    /// Counter mode.
    Ctr = 0x8,
    /// Cypher Block Chaining mode.
    Cbc = 0x10,
    /// Galois/Counter mode.
    Gcm = 0x18,
}

#[derive(Debug, Copy, Clone)]
pub enum Mode {
    /// Input data should be decrypted.
    Decrypt = 0x0,
    /// Input data should be encrypted.
    Encrypt = 0x20,
}

#[derive(Debug, Copy, Clone)]
pub enum CtrEndian {
    /// Counter should be treated as big endian (matches NIST spec).
    Big = 0x0,
    /// Counter should be treated as little endian.
    Little = 0x40,
}

pub enum AesModule {
    Reset = 0x1,
    Enable = 0x80,
}

#[derive(Debug, Clone, Copy)]
pub enum Interrupt {
    WFIFOOverflow = 0,
    RFIFOOverflow,
    RFIFOUnderflow,
    DoneCipher,
    DoneKeyExpansion,
    DoneWipeSecrets,
}

pub enum ParsedInterrupt {
    Found(Interrupt),
    None,
}

impl From<u32> for ParsedInterrupt {
    fn from(interrupt: u32) -> Self {
        match interrupt {
            104 => ParsedInterrupt::Found(Interrupt::DoneCipher),
            105 => ParsedInterrupt::Found(Interrupt::DoneKeyExpansion),
            106 => ParsedInterrupt::Found(Interrupt::DoneWipeSecrets),
            107 => ParsedInterrupt::Found(Interrupt::RFIFOOverflow),
            108 => ParsedInterrupt::Found(Interrupt::RFIFOUnderflow),
            109 => ParsedInterrupt::Found(Interrupt::WFIFOOverflow),
            _ => ParsedInterrupt::None,
        }
    }
}


pub trait AesClient {
    fn done_cipher(&self);
    fn done_key_expansion(&self);
    fn done_wipe_secrets(&self);
}
