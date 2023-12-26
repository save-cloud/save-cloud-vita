use std::{
    ffi::{c_char, CStr},
    fmt::{Display, Formatter},
    ops::Deref,
};

use crate::utils::str_to_c_str;

extern "C" {
    fn show_psv_ime(input_init: *const c_char) -> *mut c_char;
    fn ime_input_free(ime_input: *mut c_char);
    fn get_format_time() -> *mut c_char;
}

pub struct ImeInput(*mut c_char);

impl Deref for ImeInput {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        unsafe {
            if self.0.is_null() {
                return "";
            }
            let c_str = CStr::from_ptr(self.0);
            c_str.to_str().unwrap()
        }
    }
}

impl Display for ImeInput {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.deref())
    }
}

impl Drop for ImeInput {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                ime_input_free(self.0);
            }
        }
    }
}

pub fn show_keyboard(input_init: &str) -> ImeInput {
    unsafe {
        let c_str = str_to_c_str(input_init);
        ImeInput(show_psv_ime(c_str.as_slice().as_ptr() as *const c_char))
    }
}

pub fn get_current_format_time() -> ImeInput {
    unsafe { ImeInput(get_format_time()) }
}
