use core::intrinsics::copy_nonoverlapping;
use core::mem::{transmute, size_of};

pub unsafe trait Serialize: Sized {
    fn serialize(&self, buffer: &mut [u8]) -> usize {
        let length = if buffer.len() < size_of::<Self>() {
            buffer.len()
        } else {
            size_of::<Self>()
        };

        unsafe {
            copy_nonoverlapping(transmute(self), buffer.as_mut_ptr(), length);
        }
        length
    }
}

unsafe impl Serialize for u8 {}
unsafe impl Serialize for u16 {}
unsafe impl Serialize for u32 {}
unsafe impl Serialize for u64 {}
unsafe impl Serialize for usize {}
unsafe impl Serialize for i8 {}
unsafe impl Serialize for i16 {}
unsafe impl Serialize for i32 {}
unsafe impl Serialize for i64 {}
unsafe impl Serialize for isize {}
