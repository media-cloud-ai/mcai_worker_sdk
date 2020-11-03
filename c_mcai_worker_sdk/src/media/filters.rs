use crate::get_c_string;
use crate::media::stream_descriptors::{CStreamDescriptor, StreamType};
use mcai_worker_sdk::{error, GenericFilter};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::ffi::c_void;
use std::os::raw::{c_char, c_uint};

pub(crate) type NewStreamDescriptorCallback = extern "C" fn(c_uint, c_uint) -> *const c_void;
pub(crate) type AddDescriptorFilterCallback = extern "C" fn(*const c_void, *const c_void);
pub(crate) type NewFilterCallback = extern "C" fn(*const c_char, *const c_char) -> *const c_void;
pub(crate) type AddFilterParameterCallback =
  extern "C" fn(*const c_void, *const c_char, *const c_char);

pub(crate) extern "C" fn new_stream_descriptor(
  index: c_uint,
  c_stream_type: c_uint,
) -> *const c_void {
  let stream_type = StreamType::try_from(c_stream_type as u8);
  if let Err(error) = stream_type {
    error!("{}", error);
    return std::ptr::null();
  }

  let c_stream_descriptor = CStreamDescriptor {
    index,
    stream_type: stream_type.unwrap(),
    filters: vec![],
  };

  Box::into_raw(Box::new(c_stream_descriptor)) as *const c_void
}

#[allow(unused_assignments)]
pub(crate) extern "C" fn add_descriptor_filter(
  mut descriptor: *const c_void,
  filter: *const c_void,
) {
  unsafe {
    let mut c_descriptor = Box::from_raw(descriptor as *mut CStreamDescriptor);
    c_descriptor
      .filters
      .push(*Box::from_raw(filter as *mut GenericFilter));
    descriptor = Box::into_raw(c_descriptor) as *const c_void
  }
}

pub(crate) extern "C" fn new_filter(name: *const c_char, label: *const c_char) -> *const c_void {
  let filter_ptr = unsafe {
    let name = get_c_string!(name);
    let label = if label.is_null() {
      None
    } else {
      Some(get_c_string!(label))
    };

    let filter = GenericFilter {
      name,
      label,
      parameters: HashMap::new(),
    };
    Box::into_raw(Box::new(filter))
  };

  filter_ptr as *const c_void
}

#[allow(unused_assignments)]
pub(crate) extern "C" fn add_filter_parameter(
  mut filter: *const c_void,
  key: *const c_char,
  value: *const c_char,
) {
  unsafe {
    let mut c_filter = Box::from_raw(filter as *mut GenericFilter);
    c_filter
      .parameters
      .insert(get_c_string!(key), get_c_string!(value));
    filter = Box::into_raw(c_filter) as *const c_void
  }
}

#[cfg(all(test, feature = "media"))]
mod media_filter_tests {
  use crate::media::filters::{
    add_descriptor_filter, add_filter_parameter, new_filter, new_stream_descriptor,
  };
  use crate::media::stream_descriptors::{CStreamDescriptor, StreamType};
  use mcai_worker_sdk::GenericFilter;
  use std::ffi::CString;

  #[test]
  pub fn test_c_new_filter() {
    let name = "filter_name".to_string();
    let label = "filter_label".to_string();

    unsafe {
      let c_filter_ptr = new_filter(
        CString::new(name.clone()).unwrap().as_ptr(),
        CString::new(label.clone()).unwrap().as_ptr(),
      );
      let c_filter = Box::from_raw(c_filter_ptr as *mut GenericFilter);

      assert_eq!(name, c_filter.name);
      assert_eq!(Some(label), c_filter.label);
      assert_eq!(0, c_filter.parameters.len());
    }
  }

  #[test]
  pub fn test_add_filter_parameter() {
    let key = "Hello".to_string();
    let value = "World".to_string();

    let name = "filter_name".to_string();
    let label = "filter_label".to_string();

    unsafe {
      let c_filter_ptr = new_filter(
        CString::new(name.clone()).unwrap().as_ptr(),
        CString::new(label.clone()).unwrap().as_ptr(),
      );
      add_filter_parameter(
        c_filter_ptr,
        CString::new(key.clone()).unwrap().as_ptr(),
        CString::new(value.clone()).unwrap().as_ptr(),
      );
      let c_filter = Box::from_raw(c_filter_ptr as *mut GenericFilter);
      assert_eq!(name, c_filter.name);
      assert_eq!(Some(label), c_filter.label);
      assert_eq!(1, c_filter.parameters.len());
      assert_eq!(&value, c_filter.parameters.get(&key).unwrap())
    }
  }

  #[test]
  pub fn test_c_new_stream_descriptor() {
    unsafe {
      let descriptor_ptr = new_stream_descriptor(1, 1);
      let descriptor = Box::from_raw(descriptor_ptr as *mut CStreamDescriptor);
      assert_eq!(1, descriptor.index);
      assert_eq!(StreamType::Audio, descriptor.stream_type);
      assert_eq!(0, descriptor.filters.len());
    }
  }

  #[test]
  pub fn test_add_descriptor_filter() {
    let name = "filter_name".to_string();
    let label = "filter_label".to_string();

    unsafe {
      let c_filter_ptr = new_filter(
        CString::new(name.clone()).unwrap().as_ptr(),
        CString::new(label.clone()).unwrap().as_ptr(),
      );
      let descriptor_ptr = new_stream_descriptor(1, 1);

      add_descriptor_filter(descriptor_ptr, c_filter_ptr);

      let descriptor = Box::from_raw(descriptor_ptr as *mut CStreamDescriptor);
      assert_eq!(1, descriptor.index);
      assert_eq!(StreamType::Audio, descriptor.stream_type);
      assert_eq!(1, descriptor.filters.len());

      let generic_filter = descriptor.filters.get(0).unwrap();
      assert_eq!(name, generic_filter.name);
      assert_eq!(Some(label), generic_filter.label);
      assert_eq!(0, generic_filter.parameters.len());
    }
  }

  #[test]
  pub fn test_add_descriptor_filters_with_parameters() {
    let name_1 = "filter_name_1".to_string();
    let label_1 = "filter_label_1".to_string();

    let key_1 = "parameter_key_1".to_string();
    let value_1 = "parameter_value_1".to_string();

    let name_2 = "filter_name_2".to_string();
    let label_2 = "filter_label_2".to_string();

    let key_2 = "parameter_key_2".to_string();
    let value_2 = "parameter_value_2".to_string();

    unsafe {
      let descriptor_ptr = new_stream_descriptor(1, 1);

      let c_filter_ptr = new_filter(
        CString::new(name_1.clone()).unwrap().as_ptr(),
        CString::new(label_1.clone()).unwrap().as_ptr(),
      );
      add_filter_parameter(
        c_filter_ptr,
        CString::new(key_1.clone()).unwrap().as_ptr(),
        CString::new(value_1.clone()).unwrap().as_ptr(),
      );
      add_filter_parameter(
        c_filter_ptr,
        CString::new(key_2.clone()).unwrap().as_ptr(),
        CString::new(value_2.clone()).unwrap().as_ptr(),
      );

      add_descriptor_filter(descriptor_ptr, c_filter_ptr);

      let c_filter_ptr = new_filter(
        CString::new(name_2.clone()).unwrap().as_ptr(),
        CString::new(label_2.clone()).unwrap().as_ptr(),
      );
      add_filter_parameter(
        c_filter_ptr,
        CString::new(key_1.clone()).unwrap().as_ptr(),
        CString::new(value_1.clone()).unwrap().as_ptr(),
      );

      add_descriptor_filter(descriptor_ptr, c_filter_ptr);

      let descriptor = Box::from_raw(descriptor_ptr as *mut CStreamDescriptor);
      assert_eq!(1, descriptor.index);
      assert_eq!(StreamType::Audio, descriptor.stream_type);
      assert_eq!(2, descriptor.filters.len());

      let generic_filter = descriptor.filters.get(0).unwrap();
      assert_eq!(name_1, generic_filter.name);
      assert_eq!(Some(label_1), generic_filter.label);
      assert_eq!(2, generic_filter.parameters.len());
      assert_eq!(&value_1, generic_filter.parameters.get(&key_1).unwrap());
      assert_eq!(&value_2, generic_filter.parameters.get(&key_2).unwrap());

      let generic_filter = descriptor.filters.get(1).unwrap();
      assert_eq!(name_2, generic_filter.name);
      assert_eq!(Some(label_2), generic_filter.label);
      assert_eq!(1, generic_filter.parameters.len());
      assert_eq!(&value_1, generic_filter.parameters.get(&key_1).unwrap());
    }
  }
}
