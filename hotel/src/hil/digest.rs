//use main::SyscallError;

#[derive(Copy, Clone)]
pub enum DigestMode {
    /// Generates a SHA-1 digest. Output size is 160 bits (20 bytes).
    Sha1,
    /// Generates a SHA-2 256-bit digest. Output size is 256 bits (32 bytes).
    Sha256,
}

impl DigestMode {
    pub fn output_size(&self) -> usize {
        match *self {
            DigestMode::Sha1 => 160 / 8,
            DigestMode::Sha256 => 256 / 8,
        }
    }
}

pub enum DigestError {
    /// The requested digest type is not supported by this hardware.
    EngineNotSupported,
    /// `update` or `finalize` where called before `initialize`.
    NotConfigured,
    /// The supplied output buffer is too small. Parameter is the required buffer size.
    BufferTooSmall(usize),
}

/*impl From<DigestError> for SyscallError {
    fn from(e: DigestError) -> Self {
        match e {
            DigestError::EngineNotSupported => SyscallError::NotImplemented,
            DigestError::NotConfigured => SyscallError::InvalidState,
            DigestError::BufferTooSmall(_) => SyscallError::OutOfRange,
        }
    }
}*/

pub trait DigestEngine {
    /// Initializes the digest engine for the given mode.
    fn initialize(&mut self, mode: DigestMode) -> Result<(), DigestError>;

    /// Feeds data into the digest. Returns the number of bytes that were actually consumed from
    /// the input.
    fn update(&mut self, data: &[u8]) -> Result<usize, DigestError>;

    /// Finalizes the digest, and stores it in the `output` buffer. Returns the number of bytes
    /// stored.
    fn finalize(&mut self, output: &mut [u8]) -> Result<usize, DigestError>;
}
