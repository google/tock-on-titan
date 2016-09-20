pub const WAIT: u8 = 0;
pub const SUBSCRIBE: u8 = 1;
pub const COMMAND: u8 = 2;
pub const ALLOW: u8 = 3;
pub const MEMOP: u8 = 4;

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
