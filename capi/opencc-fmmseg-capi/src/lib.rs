use opencc_fmmseg::OpenCC;

#[no_mangle]
pub extern "C" fn opencc_new() -> *mut OpenCC {
    Box::into_raw(Box::new(OpenCC::new()))
}

#[no_mangle]
pub extern "C" fn opencc_free(instance: *mut OpenCC) {
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
pub extern "C" fn opencc_convert(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
    config: *const std::os::raw::c_char,
    punctuation: bool,
) -> *mut std::os::raw::c_char {
    if instance.is_null() {
        return std::ptr::null_mut(); // Return null pointer if the instance pointer is null
    }
    let opencc = unsafe { &*instance }; // Convert the instance pointer back into a reference
                                        // Convert input from C string to Rust string
    let config_c_str = unsafe { std::ffi::CStr::from_ptr(config) };
    let config_str_slice = config_c_str.to_str().unwrap_or("");
    let config_str = config_str_slice.to_owned();

    let input_c_str = unsafe { std::ffi::CStr::from_ptr(input) };
    let input_str_slice = input_c_str.to_str().unwrap_or("");
    let input_str = input_str_slice.to_owned();

    // let config_str = unsafe { CFixedStr::from_ptr(config, libc::strlen(config)).to_string_lossy() };
    // let input_str = unsafe { CFixedStr::from_ptr(input, libc::strlen(input)).to_string_lossy() };

    let result = opencc.convert(input_str.as_str(), config_str.as_str(), punctuation);

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
    opencc.zho_check(&input_str)
}

#[no_mangle]
pub extern "C" fn opencc_last_error() -> *mut std::os::raw::c_char {
    let last_error = match OpenCC::get_last_error() {
        Some(err) => err,
        None => return std::ptr::null_mut(), // Return null pointer if no error
    };
    // Convert the Rust string result to a C string
    let c_result = match std::ffi::CString::new(last_error) {
        Ok(c_str) => c_str,
        Err(_) => return std::ptr::null_mut(), // Return null pointer if CString creation fails
    };

    c_result.into_raw()
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
    fn test_opencc_invalid() {
        // Create a sample OpenCC instance
        let opencc = OpenCC::new();
        // Define a sample input string
        let input = "你好，世界，欢迎！";
        // Convert the input string to a C string
        let c_config = std::ffi::CString::new("s2s")
            .expect("CString conversion failed")
            .into_raw();
        // Convert the input string to a C string
        let c_input = std::ffi::CString::new(input)
            .expect("CString conversion failed")
            .into_raw();
        // Define the punctuation flag
        let punctuation = false;
        // Call the function under test
        let result_ptr = opencc_convert(&opencc as *const OpenCC, c_input, c_config, punctuation);
        // Convert the result C string to Rust string
        let result_str = unsafe {
            std::ffi::CString::from_raw(result_ptr)
                .to_string_lossy()
                .into_owned()
        };

        // Free the allocated C string
        // unsafe { let _ = std::ffi::CString::from_raw(result_ptr); };

        // Assert the result
        // println!("{:?}", OpenCC::get_last_error());
        assert_eq!(result_str, "");
        // assert_eq!(result_str, "你好，世界，歡迎！");
        assert_eq!(
            Some(OpenCC::get_last_error().unwrap().contains("Invalid")),
            Some(true)
        );
    }

    #[test]
    fn test_opencc_convert() {
        // Create a sample OpenCC instance
        let opencc = OpenCC::new();
        // Define a sample input string
        let input = "意大利罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";
        // Convert the input string to a C string
        let c_config = std::ffi::CString::new("s2twp")
            .expect("CString conversion failed")
            .into_raw();
        // Convert the input string to a C string
        let c_input = std::ffi::CString::new(input)
            .expect("CString conversion failed")
            .into_raw();
        // Define the punctuation flag
        let punctuation = true;
        // Call the function under test
        let result_ptr = opencc_convert(&opencc as *const OpenCC, c_input, c_config, punctuation);
        // Convert the result C string to Rust string
        let result_str = unsafe {
            std::ffi::CString::from_raw(result_ptr)
                .to_string_lossy()
                .into_owned()
        };

        // Free the allocated C string
        // unsafe { let _ = std::ffi::CString::from_raw(result_ptr); };

        // Assert the result
        assert_eq!(
            result_str,
            "義大利羅浮宮裡收藏的「蒙娜麗莎的微笑」畫像是曠世之作。"
        );
    }

    #[test]
    fn test_opencc_last_error() {
        // Call the opencc_last_error function
        let error_ptr = opencc_last_error();
        // Convert the raw pointer to a C string
        let c_string = unsafe {
            if !error_ptr.is_null() {
                std::ffi::CString::from_raw(error_ptr)
            } else {
                std::ffi::CString::new("No error").unwrap()
            }
        };
        // Convert the C string to a Rust string
        let error_message = c_string.into_string().unwrap();
        // Test the error message (replace "expected_error_message" with the expected error message)
        assert_eq!(error_message, "No error");
    }

    #[test]
    fn test_opencc_last_error_2() {
        let _opencc = OpenCC::from_json("test.json");
        let last_error_0 = OpenCC::get_last_error().unwrap_or_else(|| "No error".to_string());
        let error_ptr = opencc_last_error();

        let c_error = unsafe {
            if error_ptr.is_null() {
                std::ffi::CString::new("No error").unwrap()
            } else {
                std::ffi::CString::from_raw(error_ptr)
            }
        };
        // Convert the C string to a Rust string
        let error_message = c_error.clone().into_string().unwrap();
        println!(
            "Left: {}\nRight: {}",
            last_error_0,
            c_error.into_string().unwrap()
        );

        assert_eq!(error_message, last_error_0);
    }
}
