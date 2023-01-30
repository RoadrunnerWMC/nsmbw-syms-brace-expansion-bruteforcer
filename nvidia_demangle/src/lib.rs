extern crate libc;
extern crate nvidia_demangle_sys;

use std::ffi::{CStr, CString};
use std::str;
use std::error::Error;

use libc::c_char;


pub fn demangle_with_buf_size(s: &str, buf_size: usize) -> Result<String, Box<dyn Error>> {
    let input = CString::new(s)?;
    let mut output_vec: Vec<c_char> = vec![0; buf_size];

    unsafe {
        let ptr = input.into_raw();

        nvidia_demangle_sys::demangle(
            output_vec.as_mut_ptr(),
            buf_size.try_into()?,
            ptr);

        let output_str = CStr::from_ptr(output_vec.as_ptr());

        // prevent memory leak -- see https://doc.rust-lang.org/stable/alloc/ffi/struct.CString.html#method.into_raw
        let _ = CString::from_raw(ptr);

        Ok(output_str.to_str()?.to_owned())
    }
}


const DEFAULT_BUF_SIZE: usize = 1024;
pub fn demangle(s: &str) -> Result<String, Box<dyn Error>> {
    demangle_with_buf_size(s, DEFAULT_BUF_SIZE)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_symbol() {
        let m = "construct__10dWmActor_cFUsP7dBase_cUlPC7mVec3_cPC7mAng3_c";
        let d = "dWmActor_c::construct( unsigned short, dBase_c*, unsigned long, const mVec3_c*, const mAng3_c* )";
        assert_eq!(&demangle(m).unwrap(), d);
    }

    #[test]
    fn test_broken_symbol() {
        let m = "holdSound__18NMSndObjectCmn<12>FUlRCQ34nw4r4math4VEC2Ul";
        let d = "NMSndObjectCmn<>FUlRCQ34nw4::holdSound";
        assert_eq!(&demangle(m).unwrap(), d);
    }

    #[test]
    fn test_empty_symbol() {
        assert_eq!(&demangle("").unwrap(), "");
    }
}
