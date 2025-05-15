use opencc_fmmseg::OpenCC;

#[no_mangle]
pub extern "C" fn opencc_new() -> *mut OpenCC {
    Box::into_raw(Box::new(OpenCC::new()))
}

// #[no_mangle]
// pub extern "C" fn opencc_new_from_dicts() -> *mut OpenCC {
//     Box::into_raw(Box::new(OpenCC::from_dicts()))
// }

#[no_mangle]
pub extern "C" fn opencc_delete(instance: *mut OpenCC) {
    if !instance.is_null() {
        unsafe {
            let _ = Box::from_raw(instance);
        }
    }
}

#[deprecated(note = "Use `opencc_delete` instead")]
#[no_mangle]
pub extern "C" fn opencc_free(instance: *mut OpenCC) {
    if !instance.is_null() {
        unsafe {
            let _ = Box::from_raw(instance);
        }
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
    // Convert the instance pointer back into a reference
    let opencc = unsafe { &*instance };

    let config_c_str = unsafe { std::ffi::CStr::from_ptr(config) };
    let config_str_slice = config_c_str.to_str().unwrap_or("");

    let input_c_str = unsafe { std::ffi::CStr::from_ptr(input) };
    let input_str_slice = input_c_str.to_str().unwrap_or("");

    let result = opencc.convert(input_str_slice, config_str_slice, punctuation);

    // Try to create a CString from result. If it fails, fallback to an empty CString.
    std::ffi::CString::new(result)
        .unwrap_or_else(|_| std::ffi::CString::new("").unwrap())
        .into_raw()
}

#[no_mangle]
pub extern "C" fn opencc_convert_len(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
    input_len: usize,
    config: *const std::os::raw::c_char,
    punctuation: bool,
) -> *mut std::os::raw::c_char {
    if instance.is_null() || input.is_null() || config.is_null() {
        return std::ptr::null_mut();
    }

    let opencc = unsafe { &*instance };

    let input_slice = unsafe { std::slice::from_raw_parts(input as *const u8, input_len) };

    let input_str = match std::str::from_utf8(input_slice) {
        Ok(s) => std::borrow::Cow::Borrowed(s),
        Err(e) => {
            OpenCC::set_last_error(&format!("Invalid UTF-8 input: {}", e));
            std::borrow::Cow::Owned(String::from_utf8_lossy(input_slice).into_owned())
        }
    };

    let config_str = unsafe { std::ffi::CStr::from_ptr(config).to_str().unwrap_or("") };

    let result = opencc.convert(&*input_str, config_str, punctuation);

    std::ffi::CString::new(result)
        .unwrap_or_else(|_| std::ffi::CString::new("").unwrap())
        .into_raw()
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
    // Convert the instance pointer back into a reference
    let opencc = unsafe { &*instance };
    // Convert input from C string to Rust string
    let c_str = unsafe { std::ffi::CStr::from_ptr(input) };
    let str_slice = c_str.to_str().unwrap_or("");

    opencc.zho_check(str_slice)
}

#[no_mangle]
pub extern "C" fn opencc_last_error() -> *mut std::os::raw::c_char {
    match OpenCC::get_last_error() {
        Some(ref err) if !err.is_empty() => match std::ffi::CString::new(err.as_str()) {
            Ok(c_str) => c_str.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        _ => {
            // Return "No error" if None or empty
            std::ffi::CString::new("No error").unwrap().into_raw()
        }
    }
}

#[no_mangle]
pub extern "C" fn opencc_error_free(ptr: *mut std::os::raw::c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = std::ffi::CString::from_raw(ptr);
            // Automatically dropped and deallocated
        }
    }
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
        OpenCC::set_last_error("");
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
            std::ffi::CStr::from_ptr(result_ptr)
                .to_string_lossy()
                .into_owned()
        };
        // Free the allocated C string
        opencc_string_free(result_ptr);

        // Assert the result
        // println!("{:?}", OpenCC::get_last_error());
        assert_eq!(result_str, "Invalid config: s2s");
        // assert_eq!(result_str, "你好，世界，歡迎！");
        assert_eq!(
            Some(OpenCC::get_last_error().unwrap().contains("Invalid")),
            Some(true)
        );
        OpenCC::set_last_error("");
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
            std::ffi::CStr::from_ptr(result_ptr)
                .to_string_lossy()
                .into_owned()
        };

        // Free the allocated C string
        opencc_string_free(result_ptr);

        // Assert the result
        assert_eq!(
            result_str,
            "義大利羅浮宮裡收藏的「蒙娜麗莎的微笑」畫像是曠世之作。"
        );
    }

    #[test]
    fn test_opencc_convert_len() {
        // Create a sample OpenCC instance
        let opencc = OpenCC::new();
        // Define a sample input string
        let input = "意大利罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";
        // Get raw pointer and byte length
        let input_bytes = input.as_bytes();
        let input_len = input_bytes.len();
        let c_input_ptr = input_bytes.as_ptr() as *const std::os::raw::c_char;
        // Convert the config string to a C string
        let c_config = std::ffi::CString::new("s2twp")
            .expect("CString conversion failed")
            .into_raw();

        // Define the punctuation flag
        let punctuation = true;
        // Call the function under test (assumes a new FFI function opencc_convert_len exists)
        let result_ptr = opencc_convert_len(
            &opencc as *const OpenCC,
            c_input_ptr,
            input_len,
            c_config,
            punctuation,
        );

        // Convert the result C string to Rust string
        let result_str = unsafe {
            std::ffi::CString::from_raw(result_ptr)
                .to_string_lossy()
                .into_owned()
        };
        // Assert the result
        assert_eq!(
            result_str,
            "義大利羅浮宮裡收藏的「蒙娜麗莎的微笑」畫像是曠世之作。"
        );
    }

    #[test]
    fn test_opencc_convert_len_incomplete_utf8() {
        //Clear any previous test errors
        OpenCC::set_last_error("");

        use std::ffi::CString;

        let opencc = OpenCC::new();

        // Incomplete UTF-8 sequence: `0xE5` alone is not valid
        let broken_utf8: &[u8] = &[0xE5];
        let input_ptr = broken_utf8.as_ptr() as *const std::os::raw::c_char;
        let input_len = broken_utf8.len();

        let config = CString::new("s2twp").unwrap();
        let config_ptr = config.as_ptr();

        let result_ptr = opencc_convert_len(
            &opencc as *const OpenCC,
            input_ptr,
            input_len,
            config_ptr,
            true,
        );

        // Read and clean up result string
        let result = unsafe {
            if result_ptr.is_null() {
                "[null]".to_string()
            } else {
                let s = CString::from_raw(result_ptr).to_string_lossy().into_owned();
                s
            }
        };

        let last_error = read_and_free(opencc_last_error());

        println!("Result: {:?}", result);
        println!("Last Error: {:?}", last_error);

        assert_eq!(
            Some(OpenCC::get_last_error().unwrap().contains("Invalid")),
            Some(last_error.contains("Invalid"))
        );
    }
    #[test]
    fn test_opencc_last_error() {
        // Clear any previous global error
        OpenCC::set_last_error("");
        // Convert the raw pointer to a Rust string (clone first, then free)
        let error_message = read_and_free(opencc_last_error());

        // Assert that the error message is "No error"
        assert_eq!(error_message, "No error");

        // Optionally, verify that the LAST_ERROR is reset
        assert_eq!(
            OpenCC::get_last_error().unwrap_or_else(|| "No error".to_string()),
            ""
        );
    }

    #[test]
    fn test_opencc_last_error_2() {
        // Clear any previous global error to prevent contamination from previous tests
        OpenCC::set_last_error("");

        let _opencc = OpenCC::from_cbor("test.json");

        // Get the last error message before calling opencc_last_error
        let last_error_0 = OpenCC::get_last_error().unwrap_or_else(|| "No error".to_string());

        // Convert the raw pointer to a Rust String safely, then free
        let error_message = read_and_free(opencc_last_error());
        assert_eq!(error_message, last_error_0);

        // Compare the error message
        println!("Left: {}\nRight: {}", last_error_0, error_message);
        assert_eq!(error_message, last_error_0);

        // Optionally, verify that the LAST_ERROR is reset
        assert_eq!(
            OpenCC::get_last_error().unwrap_or_else(|| "No error".to_string()),
            last_error_0
        );
    }

    fn read_and_free(ptr: *mut std::os::raw::c_char) -> String {
        unsafe {
            if ptr.is_null() {
                "[null]".to_string()
            } else {
                let msg = std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned();
                opencc_error_free(ptr);
                msg
            }
        }
    }
}
