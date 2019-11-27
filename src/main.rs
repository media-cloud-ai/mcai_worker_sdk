use std::ffi::CStr;
use std::os::raw::c_char;

extern "C" {
  fn get_name() -> *const c_char;
  fn get_short_description() -> *const c_char;
  fn get_description() -> *const c_char;
  fn get_version() -> *const c_char;
}

fn main() {
  unsafe {
    let name = CStr::from_ptr(get_name());
    let short_description = CStr::from_ptr(get_short_description());
    let description = CStr::from_ptr(get_description());
    let version = CStr::from_ptr(get_version());

    println!("Name: {:?}", name);
    println!("Short description: {:?}", short_description);
    println!("Description: {:?}", description);
    println!("Version: {:?}", version);
  }
}
