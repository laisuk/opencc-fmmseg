use c_fixed_string::CFixedStr;
use opencc_fmmseg::OpenCC;

#[no_mangle]
pub extern "C" fn opencc_new() -> *mut OpenCC {
    Box::into_raw(Box::new(OpenCC::new()))
}

#[no_mangle]
pub extern "C" fn opencc_close(instance: *mut OpenCC) {
    if !instance.is_null() {
        // Convert the raw pointer back into a Box and let it drop
        unsafe {
            let _ = Box::from_raw(instance);
        };
    }
}

#[no_mangle]
pub extern "C" fn opencc_get_parallel(instance: *mut OpenCC) -> bool {
    let opencc = unsafe { &*instance };
    opencc.get_parallel()
}

#[no_mangle]
pub extern "C" fn opencc_set_parallel(instance: *mut OpenCC, is_parallel: bool) {
    let opencc = unsafe { &mut *instance };
    opencc.set_parallel(is_parallel);
}

#[no_mangle]
pub extern "C" fn opencc_s2t(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
    punctuation: bool,
) -> *mut std::os::raw::c_char {
    if instance.is_null() {
        return std::ptr::null_mut(); // Return null pointer if the instance pointer is null
    }

    let opencc = unsafe { &*instance }; // Convert the instance pointer back into a reference

    // Convert input from C string to Rust string
    // let c_str = unsafe { std::ffi::CStr::from_ptr(input) };
    // let str_slice = c_str.to_str().unwrap_or("");
    // let input_str = str_slice.to_owned();

    let input_str = unsafe { CFixedStr::from_ptr(input, libc::strlen(input)).to_string_lossy() };

    let result = opencc.s2t(&input_str, punctuation);

    // Convert the Rust string result to a C string
    let c_result = std::ffi::CString::new(result).unwrap();
    c_result.into_raw()
}

#[no_mangle]
pub extern "C" fn opencc_s2tw(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
    punctuation: bool,
) -> *mut std::os::raw::c_char {
    if instance.is_null() {
        return std::ptr::null_mut(); // Return null pointer if the instance pointer is null
    }

    let opencc = unsafe { &*instance }; // Convert the instance pointer back into a reference
    let input_str = unsafe { CFixedStr::from_ptr(input, libc::strlen(input)).to_string_lossy() };
    let result = opencc.s2tw(&input_str, punctuation);

    // Convert the Rust string result to a C string
    let c_result = std::ffi::CString::new(result).unwrap();
    c_result.into_raw()
}

#[no_mangle]
pub extern "C" fn opencc_s2twp(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
    punctuation: bool,
) -> *mut std::os::raw::c_char {
    if instance.is_null() {
        return std::ptr::null_mut(); // Return null pointer if the instance pointer is null
    }

    let opencc = unsafe { &*instance }; // Convert the instance pointer back into a reference
    let input_str = unsafe { CFixedStr::from_ptr(input, libc::strlen(input)).to_string_lossy() };
    let result = opencc.s2twp(&input_str, punctuation);

    // Convert the Rust string result to a C string
    let c_result = std::ffi::CString::new(result).unwrap();
    c_result.into_raw()
}

#[no_mangle]
pub extern "C" fn opencc_s2hk(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
    punctuation: bool,
) -> *mut std::os::raw::c_char {
    if instance.is_null() {
        return std::ptr::null_mut(); // Return null pointer if the instance pointer is null
    }

    let opencc = unsafe { &*instance }; // Convert the instance pointer back into a reference
    let input_str = unsafe { CFixedStr::from_ptr(input, libc::strlen(input)).to_string_lossy() };
    let result = opencc.s2hk(&input_str, punctuation);

    // Convert the Rust string result to a C string
    let c_result = std::ffi::CString::new(result).unwrap();
    c_result.into_raw()
}

#[no_mangle]
pub extern "C" fn opencc_t2s(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
    punctuation: bool,
) -> *mut std::os::raw::c_char {
    if instance.is_null() {
        return std::ptr::null_mut(); // Return null pointer if the instance pointer is null
    }

    let opencc = unsafe { &*instance }; // Convert the instance pointer back into a reference
    let input_str = unsafe { CFixedStr::from_ptr(input, libc::strlen(input)).to_string_lossy() };
    let result = opencc.t2s(&input_str, punctuation);

    // Convert the Rust string result to a C string
    let c_result = std::ffi::CString::new(result).unwrap();
    c_result.into_raw()
}

#[no_mangle]
pub extern "C" fn opencc_t2tw(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    if instance.is_null() {
        return std::ptr::null_mut(); // Return null pointer if the instance pointer is null
    }

    let opencc = unsafe { &*instance }; // Convert the instance pointer back into a reference
    let input_str = unsafe { CFixedStr::from_ptr(input, libc::strlen(input)).to_string_lossy() };
    let result = opencc.t2tw(&input_str);

    // Convert the Rust string result to a C string
    let c_result = std::ffi::CString::new(result).unwrap();
    c_result.into_raw()
}

#[no_mangle]
pub extern "C" fn opencc_t2hk(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    if instance.is_null() {
        return std::ptr::null_mut(); // Return null pointer if the instance pointer is null
    }

    let opencc = unsafe { &*instance }; // Convert the instance pointer back into a reference
    let input_str = unsafe { CFixedStr::from_ptr(input, libc::strlen(input)).to_string_lossy() };
    let result = opencc.t2hk(&input_str);

    // Convert the Rust string result to a C string
    let c_result = std::ffi::CString::new(result).unwrap();
    c_result.into_raw()
}

#[no_mangle]
pub extern "C" fn opencc_tw2s(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
    punctuation: bool,
) -> *mut std::os::raw::c_char {
    if instance.is_null() {
        return std::ptr::null_mut(); // Return null pointer if the instance pointer is null
    }

    let opencc = unsafe { &*instance }; // Convert the instance pointer back into a reference
    let input_str = unsafe { CFixedStr::from_ptr(input, libc::strlen(input)).to_string_lossy() };
    let result = opencc.tw2s(&input_str, punctuation);

    // Convert the Rust string result to a C string
    let c_result = std::ffi::CString::new(result).unwrap();
    c_result.into_raw()
}

#[no_mangle]
pub extern "C" fn opencc_tw2sp(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
    punctuation: bool,
) -> *mut std::os::raw::c_char {
    if instance.is_null() {
        return std::ptr::null_mut(); // Return null pointer if the instance pointer is null
    }

    let opencc = unsafe { &*instance }; // Convert the instance pointer back into a reference
    let input_str = unsafe { CFixedStr::from_ptr(input, libc::strlen(input)).to_string_lossy() };
    let result = opencc.tw2sp(&input_str, punctuation);

    // Convert the Rust string result to a C string
    let c_result = std::ffi::CString::new(result).unwrap();
    c_result.into_raw()
}

#[no_mangle]
pub extern "C" fn opencc_tw2t(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    if instance.is_null() {
        return std::ptr::null_mut(); // Return null pointer if the instance pointer is null
    }

    let opencc = unsafe { &*instance }; // Convert the instance pointer back into a reference
    let input_str = unsafe { CFixedStr::from_ptr(input, libc::strlen(input)).to_string_lossy() };
    let result = opencc.tw2t(&input_str);

    // Convert the Rust string result to a C string
    let c_result = std::ffi::CString::new(result).unwrap();
    c_result.into_raw()
}

#[no_mangle]
pub extern "C" fn opencc_tw2tp(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    if instance.is_null() {
        return std::ptr::null_mut(); // Return null pointer if the instance pointer is null
    }

    let opencc = unsafe { &*instance }; // Convert the instance pointer back into a reference
    let input_str = unsafe { CFixedStr::from_ptr(input, libc::strlen(input)).to_string_lossy() };
    let result = opencc.tw2tp(&input_str);

    // Convert the Rust string result to a C string
    let c_result = std::ffi::CString::new(result).unwrap();
    c_result.into_raw()
}

#[no_mangle]
pub extern "C" fn opencc_hk2s(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
    punctuation: bool,
) -> *mut std::os::raw::c_char {
    if instance.is_null() {
        return std::ptr::null_mut(); // Return null pointer if the instance pointer is null
    }

    let opencc = unsafe { &*instance }; // Convert the instance pointer back into a reference
    let input_str = unsafe { CFixedStr::from_ptr(input, libc::strlen(input)).to_string_lossy() };
    let result = opencc.hk2s(&input_str, punctuation);

    // Convert the Rust string result to a C string
    let c_result = std::ffi::CString::new(result).unwrap();
    c_result.into_raw()
}

#[no_mangle]
pub extern "C" fn opencc_hk2t(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    if instance.is_null() {
        return std::ptr::null_mut(); // Return null pointer if the instance pointer is null
    }

    let opencc = unsafe { &*instance }; // Convert the instance pointer back into a reference
    let input_str = unsafe { CFixedStr::from_ptr(input, libc::strlen(input)).to_string_lossy() };
    let result = opencc.hk2t(&input_str);

    // Convert the Rust string result to a C string
    let c_result = std::ffi::CString::new(result).unwrap();
    c_result.into_raw()
}

#[no_mangle]
pub extern "C" fn opencc_jp2t(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    if instance.is_null() {
        return std::ptr::null_mut(); // Return null pointer if the instance pointer is null
    }

    let opencc = unsafe { &*instance }; // Convert the instance pointer back into a reference
    let input_str = unsafe { CFixedStr::from_ptr(input, libc::strlen(input)).to_string_lossy() };
    let result = opencc.jp2t(&input_str);

    // Convert the Rust string result to a C string
    let c_result = std::ffi::CString::new(result).unwrap();
    c_result.into_raw()
}

#[no_mangle]
pub extern "C" fn opencc_t2jp(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    if instance.is_null() {
        return std::ptr::null_mut(); // Return null pointer if the instance pointer is null
    }

    let opencc = unsafe { &*instance }; // Convert the instance pointer back into a reference
    let input_str = unsafe { CFixedStr::from_ptr(input, libc::strlen(input)).to_string_lossy() };
    let result = opencc.t2jp(&input_str);

    // Convert the Rust string result to a C string
    let c_result = std::ffi::CString::new(result).unwrap();
    c_result.into_raw()
}

// Remember to free the memory allocated for the result string from C code
#[no_mangle]
pub extern "C" fn opencc_string_free(ptr: *mut std::os::raw::c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = std::ffi::CString::from_raw(ptr);
        };
    }
}

#[no_mangle]
pub extern "C" fn opencc_zho_check(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
) -> i32 {
    if instance.is_null() {
        return -1; // Return an error code if the instance pointer is null
    }
    let opencc = unsafe { &*instance }; // Convert the instance pointer back into a reference
                                        // Convert input from C string to Rust string
    let c_str = unsafe { std::ffi::CStr::from_ptr(input) };
    let str_slice = c_str.to_str().unwrap_or("");
    let input_str = str_slice.to_owned();
    opencc.zho_check(&input_str) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opencc_zho_check() {
        // Create a sample OpenCC instance
        let opencc = OpenCC::new();

        // Define a sample input string
        let input = "你好，世界，欢迎"; // Chinese characters meaning "Hello, world!"

        // Convert the input string to a C string
        let c_input = std::ffi::CString::new(input)
            .expect("CString conversion failed")
            .into_raw();

        // Call the function under test
        let result = opencc_zho_check(&opencc as *const OpenCC, c_input);

        // Free the allocated C string
        unsafe {
            let _ = std::ffi::CString::from_raw(c_input);
        };

        // Assert the result
        assert_eq!(result, 2); // Assuming the input string is in simplified Chinese, so the result should be 2
    }

    #[test]
    fn test_opencc_s2t() {
        // Create a sample OpenCC instance
        let opencc = OpenCC::new();

        // Define a sample input string
        let input = "你好，世界，欢迎！";

        // Convert the input string to a C string
        let c_input = std::ffi::CString::new(input)
            .expect("CString conversion failed")
            .into_raw();

        // Define the punctuation flag
        let punctuation = false;

        // Call the function under test
        let result_ptr = opencc_s2t(&opencc as *const OpenCC, c_input, punctuation);

        // Convert the result C string to Rust string
        let result_str = unsafe {
            std::ffi::CString::from_raw(result_ptr)
                .to_string_lossy()
                .into_owned()
        };

        // Free the allocated C string
        // unsafe { let _ = std::ffi::CString::from_raw(result_ptr); };

        // Assert the result
        assert_eq!(result_str, "你好，世界，歡迎！");
    }
}
