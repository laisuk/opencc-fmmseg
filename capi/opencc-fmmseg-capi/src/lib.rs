use opencc_fmmseg::OpenccConfig;
use opencc_fmmseg::OpenCC;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

const OPENCC_ABI_NUMBER: u32 = 1;

/// Returns the C ABI version number.
/// This value changes ONLY when the C ABI is broken.
#[no_mangle]
pub extern "C" fn opencc_abi_number() -> u32 {
    OPENCC_ABI_NUMBER
}

/// Returns the OpenCC-FMMSEG version string (UTF-8, null-terminated).
/// Example: "0.8.4"
///
/// The returned pointer is valid for the lifetime of the program.
#[no_mangle]
pub extern "C" fn opencc_version_string() -> *const c_char {
    // Compile-time version from Cargo.toml
    static VERSION: &str = env!("CARGO_PKG_VERSION");

    // Leak once, safe by design (process lifetime)
    static mut CSTR: *const c_char = ptr::null();

    unsafe {
        if CSTR.is_null() {
            CSTR = CString::new(VERSION).unwrap().into_raw();
        }
        CSTR
    }
}

/// C API function `opencc_new`.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_new() -> *mut OpenCC {
    Box::into_raw(Box::new(OpenCC::new()))
}

/// C API function `opencc_delete`.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_delete(instance: *mut OpenCC) {
    if !instance.is_null() {
        unsafe {
            let _ = Box::from_raw(instance);
        }
    }
}

#[deprecated(note = "Use `opencc_delete` instead")]
/// C API function `opencc_free`.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_free(instance: *mut OpenCC) {
    if !instance.is_null() {
        unsafe {
            let _ = Box::from_raw(instance);
        }
    }
}

/// C API function `opencc_get_parallel`.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_get_parallel(instance: *mut OpenCC) -> bool {
    let opencc = unsafe { &*instance };
    opencc.get_parallel()
}

/// C API function `opencc_set_parallel`.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_set_parallel(instance: *mut OpenCC, is_parallel: bool) {
    let opencc = unsafe { &mut *instance };
    opencc.set_parallel(is_parallel);
}

/// C API function `opencc_convert`.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_convert(
    instance: *const OpenCC,
    input: *const c_char,
    config: *const c_char,
    punctuation: bool,
) -> *mut c_char {
    if config.is_null() {
        OpenCC::set_last_error("Invalid argument: config is NULL");
        return ptr::null_mut();
    }

    convert_core(instance, input, punctuation, || {
        let config_str =
            decode_utf8(config, "config").map_err(|_| "Invalid UTF-8 config string".to_string())?;
        OpenccConfig::try_from(config_str).map_err(|_| format!("Invalid config: {}", config_str))
    })
}

// Available since v0.8.4
/// C API function `opencc_convert_cfg`.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_convert_cfg(
    instance: *const OpenCC,
    input: *const c_char,
    config: u32,
    punctuation: bool,
) -> *mut c_char {
    convert_core(instance, input, punctuation, || {
        OpenccConfig::from_ffi(config).ok_or_else(|| format!("Invalid config: {}", config))
    })
}

// Available since v0.8.4
/// C API function `opencc_convert_cfg_mem`.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_convert_cfg_mem(
    instance: *const OpenCC,
    input: *const c_char,
    config: u32,
    punctuation: bool,
    out_buf: *mut c_char,
    out_cap: usize,
    out_required: *mut usize,
) -> bool {
    // Must be able to report required size.
    if out_required.is_null() {
        return false;
    }

    /// Writes UTF-8 bytes + trailing '\0' to out_buf.
    /// Always writes `*out_required = required` (bytes.len() + 1).
    ///
    /// Returns:
    /// - Ok(()) if buffer was sufficient OR this is a size-query (out_buf null / out_cap 0)
    /// - Err(()) if buffer is provided but too small
    #[inline]
    unsafe fn write_output_bytes(
        bytes: &[u8],
        out_buf: *mut c_char,
        out_cap: usize,
        out_required: *mut usize,
    ) -> Result<(), ()> {
        let required = bytes.len() + 1; // + '\0'
        *out_required = required;

        // size-query mode: report required only
        if out_buf.is_null() || out_cap == 0 {
            return Ok(());
        }

        if out_cap < required {
            return Err(());
        }

        ptr::copy_nonoverlapping(bytes.as_ptr(), out_buf as *mut u8, bytes.len());
        *out_buf.add(bytes.len()) = 0;
        Ok(())
    }

    /// Convenience: set last_error and try to write message into out buffer.
    #[inline]
    fn fail_with_msg(
        msg: &str,
        out_buf: *mut c_char,
        out_cap: usize,
        out_required: *mut usize,
    ) -> bool {
        OpenCC::set_last_error(msg);
        let bytes = msg.as_bytes();

        // If the error message itself contains interior NUL (shouldn't, but be safe),
        // fall back to a static message that cannot contain NUL.
        let safe_bytes = if bytes.iter().any(|&b| b == 0) {
            b"Error"
        } else {
            bytes
        };

        let write_ok =
            unsafe { write_output_bytes(safe_bytes, out_buf, out_cap, out_required).is_ok() };

        // Even if we managed to write the error message (or size-query), it's still a failure.
        // If buffer was too small, override last_error for that specific failure mode.
        if !write_ok && !(out_buf.is_null() || out_cap == 0) {
            OpenCC::set_last_error("Output buffer too small");
        }
        false
    }

    // Validate pointers
    if instance.is_null() || input.is_null() {
        return fail_with_msg(
            "Invalid argument: instance or input is NULL",
            out_buf,
            out_cap,
            out_required,
        );
    }

    // Strict UTF-8 decode for input (do NOT swallow invalid UTF-8 into "")
    let input_str = match unsafe { CStr::from_ptr(input) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            return fail_with_msg("Invalid UTF-8 input", out_buf, out_cap, out_required);
        }
    };

    // Parse config (u32 -> enum)
    let cfg = match OpenccConfig::from_ffi(config) {
        Some(c) => c,
        None => {
            let msg = format!("Invalid config: {}", config);
            return fail_with_msg(&msg, out_buf, out_cap, out_required);
        }
    };

    // Convert
    let opencc = unsafe { &*instance };
    let output_string = opencc.convert_with_config(input_str, cfg, punctuation);

    // Guard: output must not contain interior NUL (otherwise C string semantics break)
    if output_string.as_bytes().iter().any(|&b| b == 0) {
        return fail_with_msg("Output contains NUL byte", out_buf, out_cap, out_required);
    }

    // Try to write output to buffer / or size-query.
    let write_res =
        unsafe { write_output_bytes(output_string.as_bytes(), out_buf, out_cap, out_required) };

    match write_res {
        Ok(()) => {
            // True success: clear stale last_error (including size-query success)
            OpenCC::clear_last_error();
            true
        }
        Err(()) => {
            // Buffer provided but too small
            OpenCC::set_last_error("Output buffer too small");
            false
        }
    }
}

// ------ Core Shared Helpers ------

#[inline]
fn make_c_string_or_fallback(s: &str, fallback: &'static str) -> *mut c_char {
    CString::new(s)
        .unwrap_or_else(|_| CString::new(fallback).expect("static has no NUL"))
        .into_raw()
}

#[inline]
fn fail(msg: &str) -> *mut c_char {
    OpenCC::set_last_error(msg);
    make_c_string_or_fallback(msg, "Error")
}

#[inline]
fn decode_utf8<'a>(ptr_: *const c_char, what: &'static str) -> Result<&'a str, *mut c_char> {
    let s = unsafe { CStr::from_ptr(ptr_) };
    match s.to_str() {
        Ok(v) => Ok(v),
        Err(_) => Err(fail(match what {
            "input" => "Invalid UTF-8 input",
            "config" => "Invalid UTF-8 config string",
            _ => "Invalid UTF-8 string",
        })),
    }
}

/// Shared core: resolve config -> convert -> return heap C string.
/// `resolve_cfg` returns Ok(cfg) for success, Err(error_message) for user-facing errors.
#[inline]
fn convert_core<F>(
    instance: *const OpenCC,
    input: *const c_char,
    punctuation: bool,
    resolve_cfg: F,
) -> *mut c_char
where
    F: FnOnce() -> Result<OpenccConfig, String>,
{
    if instance.is_null() || input.is_null() {
        // match your existing behavior: return NULL; (or choose fail("...") if you prefer)
        OpenCC::set_last_error("Invalid argument: instance/input is NULL");
        return ptr::null_mut();
    }

    let opencc = unsafe { &*instance };

    let input_str = match decode_utf8(input, "input") {
        Ok(v) => v,
        Err(p) => return p,
    };

    let cfg = match resolve_cfg() {
        Ok(c) => c,
        Err(msg) => return fail(&msg),
    };

    let result = opencc.convert_with_config(input_str, cfg, punctuation);

    // IMPORTANT: only clear last_error if we're truly returning a valid CString success.
    match CString::new(result) {
        Ok(cstr) => {
            OpenCC::clear_last_error();
            cstr.into_raw()
        }
        Err(_) => fail("Output contains NUL byte"),
    }
}

#[deprecated(note = "Use `opencc_convert()` or `opencc_convert_cfg` instead")]
/// C API function `opencc_convert_len`.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_convert_len(
    instance: *const OpenCC,
    input: *const c_char,
    input_len: usize,
    config: *const c_char,
    punctuation: bool,
) -> *mut c_char {
    // Align with convert()/convert_cfg(): NULL args are programmer errors => NULL + last_error
    if instance.is_null() || input.is_null() || config.is_null() {
        OpenCC::set_last_error("Invalid argument: instance/input/config is NULL");
        return ptr::null_mut();
    }

    let opencc = unsafe { &*instance };

    // SAFETY: caller provided explicit length
    let input_bytes = unsafe { std::slice::from_raw_parts(input as *const u8, input_len) };

    // Strict UTF-8, no lossy recovery (align with decode_utf8() gate behavior)
    let input_str = match std::str::from_utf8(input_bytes) {
        Ok(s) => s,
        Err(_) => return fail("Invalid UTF-8 input"),
    };

    // Strict UTF-8 for config, same error message as decode_utf8("config")
    let config_str = match unsafe { CStr::from_ptr(config) }.to_str() {
        Ok(s) => s,
        Err(_) => return fail("Invalid UTF-8 config string"),
    };

    // Same config validation semantics as opencc_convert()
    let cfg = match OpenccConfig::try_from(config_str) {
        Ok(c) => c,
        Err(_) => return fail(&format!("Invalid config: {}", config_str)),
    };

    // Convert
    let result = opencc.convert_with_config(input_str, cfg, punctuation);

    // Same NUL gate as convert_core()
    match CString::new(result) {
        Ok(cstr) => {
            OpenCC::clear_last_error();
            cstr.into_raw()
        }
        Err(_) => fail("Output contains NUL byte"),
    }
}

// Remember to free the memory allocated for the result string from C code
/// C API function `opencc_string_free`.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        };
    }
}

/// C API function `opencc_zho_check`.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_zho_check(instance: *const OpenCC, input: *const c_char) -> i32 {
    if instance.is_null() {
        return -1; // Return an error code if the instance pointer is null
    }
    // Convert the instance pointer back into a reference
    let opencc = unsafe { &*instance };
    // Convert input from C string to Rust string
    let c_str = unsafe { CStr::from_ptr(input) };
    let str_slice = c_str.to_str().unwrap_or("");

    opencc.zho_check(str_slice)
}

/// C API function `opencc_last_error`.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_last_error() -> *mut c_char {
    // Contract:
    // - Always returns a heap-allocated NUL-terminated string
    // - Caller must free via opencc_error_free()
    // - Returns "No error" when there is no error
    let msg: String = match OpenCC::get_last_error() {
        Some(err) if !err.is_empty() => err,
        _ => "No error".to_string(),
    };

    // Never panic across FFI boundary
    CString::new(msg)
        .unwrap_or_else(|_| CString::new("No error").unwrap())
        .into_raw()
}

// Available since v0.8.4
/// C API function `opencc_clear_last_error`.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_clear_last_error() {
    OpenCC::clear_last_error();
}

/// C API function `opencc_error_free`.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_error_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
            // Automatically dropped and deallocated
        }
    }
}

// ------ Config Enum Helpers ------

// Available since v0.8.4
/// C API function `opencc_config_name_to_id`.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_config_name_to_id(name_utf8: *const c_char, out_id: *mut u32) -> u8 {
    if name_utf8.is_null() || out_id.is_null() {
        return 0;
    }

    let s = unsafe { CStr::from_ptr(name_utf8) };
    let bytes = s.to_bytes();

    // ASCII-only canonical names: s2t, s2tw, ...
    // Case-insensitive without allocation:
    let id = match_ascii_config_name(bytes);

    match id {
        Some(v) => {
            unsafe {
                *out_id = v;
            }
            1
        }
        None => 0,
    }
}

#[inline]
fn match_ascii_config_name(bytes: &[u8]) -> Option<u32> {
    // Optional: reject weird bytes quickly
    if !bytes.iter().all(|&b| b.is_ascii_alphanumeric()) {
        return None;
    }

    // Note: keep this table as the single owner (same as enum values).
    if eq_ascii_ci(bytes, b"s2t") {
        return Some(1);
    }
    if eq_ascii_ci(bytes, b"s2tw") {
        return Some(2);
    }
    if eq_ascii_ci(bytes, b"s2twp") {
        return Some(3);
    }
    if eq_ascii_ci(bytes, b"s2hk") {
        return Some(4);
    }
    if eq_ascii_ci(bytes, b"t2s") {
        return Some(5);
    }
    if eq_ascii_ci(bytes, b"t2tw") {
        return Some(6);
    }
    if eq_ascii_ci(bytes, b"t2twp") {
        return Some(7);
    }
    if eq_ascii_ci(bytes, b"t2hk") {
        return Some(8);
    }
    if eq_ascii_ci(bytes, b"tw2s") {
        return Some(9);
    }
    if eq_ascii_ci(bytes, b"tw2sp") {
        return Some(10);
    }
    if eq_ascii_ci(bytes, b"tw2t") {
        return Some(11);
    }
    if eq_ascii_ci(bytes, b"tw2tp") {
        return Some(12);
    }
    if eq_ascii_ci(bytes, b"hk2s") {
        return Some(13);
    }
    if eq_ascii_ci(bytes, b"hk2t") {
        return Some(14);
    }
    if eq_ascii_ci(bytes, b"jp2t") {
        return Some(15);
    }
    if eq_ascii_ci(bytes, b"t2jp") {
        return Some(16);
    }

    None
}

#[inline]
fn eq_ascii_ci(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .all(|(&x, &y)| x.to_ascii_lowercase() == y)
}

// Available since v0.8.4
/// C API function `opencc_config_id_to_name`.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_config_id_to_name(id: u32) -> *const c_char {
    // Return pointers to static NUL-terminated strings.
    // These are safe to hand out across FFI forever.
    match id {
        1 => b"s2t\0".as_ptr() as *const c_char,
        2 => b"s2tw\0".as_ptr() as *const c_char,
        3 => b"s2twp\0".as_ptr() as *const c_char,
        4 => b"s2hk\0".as_ptr() as *const c_char,
        5 => b"t2s\0".as_ptr() as *const c_char,
        6 => b"t2tw\0".as_ptr() as *const c_char,
        7 => b"t2twp\0".as_ptr() as *const c_char,
        8 => b"t2hk\0".as_ptr() as *const c_char,
        9 => b"tw2s\0".as_ptr() as *const c_char,
        10 => b"tw2sp\0".as_ptr() as *const c_char,
        11 => b"tw2t\0".as_ptr() as *const c_char,
        12 => b"tw2tp\0".as_ptr() as *const c_char,
        13 => b"hk2s\0".as_ptr() as *const c_char,
        14 => b"hk2t\0".as_ptr() as *const c_char,
        15 => b"jp2t\0".as_ptr() as *const c_char,
        16 => b"t2jp\0".as_ptr() as *const c_char,
        _ => ptr::null(),
    }
}

// ------ C API Tests ------

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
        let c_input = CString::new(input)
            .expect("CString conversion failed")
            .into_raw();
        // Call the function under test
        let result = opencc_zho_check(&opencc as *const OpenCC, c_input);
        // Free the allocated C string
        unsafe {
            let _ = CString::from_raw(c_input);
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
        let c_config = CString::new("s2s")
            .expect("CString conversion failed")
            .into_raw();
        // Convert the input string to a C string
        let c_input = CString::new(input)
            .expect("CString conversion failed")
            .into_raw();
        // Define the punctuation flag
        let punctuation = false;
        // Call the function under test
        let result_ptr = opencc_convert(&opencc as *const OpenCC, c_input, c_config, punctuation);
        // Convert the result C string to Rust string
        let result_str = unsafe { CStr::from_ptr(result_ptr).to_string_lossy().into_owned() };
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
        let c_config = CString::new("s2twp")
            .expect("CString conversion failed")
            .into_raw();
        // Convert the input string to a C string
        let c_input = CString::new(input)
            .expect("CString conversion failed")
            .into_raw();
        // Define the punctuation flag
        let punctuation = true;
        // Call the function under test
        let result_ptr = opencc_convert(&opencc as *const OpenCC, c_input, c_config, punctuation);
        // Convert the result C string to Rust string
        let result_str = unsafe { CStr::from_ptr(result_ptr).to_string_lossy().into_owned() };

        // Free the allocated C string
        opencc_string_free(result_ptr);

        // Assert the result
        assert_eq!(
            result_str,
            "義大利羅浮宮裡收藏的「蒙娜麗莎的微笑」畫像是曠世之作。"
        );
    }

    #[test]
    fn test_opencc_convert_cfg() {
        // Create a sample OpenCC instance
        let opencc = OpenCC::new();

        // Define a sample input string
        let input = "意大利罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";

        // Convert the input string to a C string (no leak: keep CString alive)
        let c_input = CString::new(input).expect("CString conversion failed");

        // Use numeric config (OpenccConfig::S2twp == 3)
        let config: u32 = OpenccConfig::S2twp as u32;

        // Define the punctuation flag
        let punctuation = true;

        // Call the function under test
        let result_ptr = opencc_convert_cfg(
            &opencc as *const OpenCC,
            c_input.as_ptr(),
            config,
            punctuation,
        );

        // Convert the result C string to Rust string
        let result_str = unsafe { CStr::from_ptr(result_ptr).to_string_lossy().into_owned() };

        // Free the allocated output C string
        opencc_string_free(result_ptr);

        // Assert the result
        assert_eq!(
            result_str,
            "義大利羅浮宮裡收藏的「蒙娜麗莎的微笑」畫像是曠世之作。"
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

    fn read_and_free(ptr: *mut c_char) -> String {
        unsafe {
            if ptr.is_null() {
                "[null]".to_string()
            } else {
                let msg = CStr::from_ptr(ptr).to_string_lossy().into_owned();
                opencc_error_free(ptr);
                msg
            }
        }
    }

    #[test]
    fn opencc_abi_number_is_non_zero_and_stable() {
        let abi = opencc_abi_number();

        // ABI must be non-zero
        assert!(abi > 0, "ABI number must be non-zero");

        // Optional: lock current ABI if you want strict guarantee
        assert_eq!(abi, 1, "Unexpected OpenCC C API ABI number");
    }

    #[test]
    fn opencc_version_string_is_valid_utf8_and_non_empty() {
        use std::ffi::CStr;

        let ptr = opencc_version_string();
        assert!(!ptr.is_null(), "Version string pointer must not be null");

        let ver = unsafe { CStr::from_ptr(ptr) }
            .to_str()
            .expect("Version string must be valid UTF-8");

        assert!(!ver.is_empty(), "Version string must not be empty");
    }
}