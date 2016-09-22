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

impl From<DigestError> for SyscallError {
    fn from(e: DigestError) -> Self {
        match e {
            DigestError::EngineNotSupported => SyscallError::NotImplemented,
            DigestError::NotConfigured => SyscallError::InvalidState,
            DigestError::BufferTooSmall(_) => SyscallError::OutOfRange,
        }
    }
}

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

/// Possible errors returned by syscalls. In case of failure, the negative value of the error is
/// returned to the application.
#[derive(Copy, Clone)]
pub enum SyscallError {
    /// Generic errors that haven't been updated to use a more specific code yet.
    Unknown = 1,
    /// An argument passed is not (and never is) valid for this particular call.
    InvalidArgument = 2,
    /// An argument passed or operation attempted is not valid for the current state of the object.
    InvalidState = 3,
    /// A numeric argument is out-of-range, or a passed buffer is too small.
    OutOfRange = 4,
    /// The requested operation is unknown or unsupported.
    NotImplemented = 5,
    /// The resource is currently busy.
    ResourceBusy = 6,
    /// Internal error in the kernel. This indicates a bug and that the kernel might be unstable.
    InternalError = 7,
}

impl From<SyscallError> for isize {
    fn from(e: SyscallError) -> Self {
        -(e as isize)
    }
}
