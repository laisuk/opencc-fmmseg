use opencc_fmmseg::{OpenCC, OpenccConfig};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

const OPENCC_ABI_NUMBER: u32 = 1;

// ============================================================================
// Version / ABI
// ============================================================================

/// Returns the C ABI version number.
///
/// This value changes only when the C ABI is broken.
#[no_mangle]
pub extern "C" fn opencc_abi_number() -> u32 {
    OPENCC_ABI_NUMBER
}

/// Returns the OpenCC-FMMSEG version string (UTF-8, null-terminated).
///
/// Example: `"0.9.1"`.
///
/// The returned pointer is valid for the lifetime of the program.
#[no_mangle]
pub extern "C" fn opencc_version_string() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

// ============================================================================
// Instance lifetime
// ============================================================================

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
    free_opencc_instance(instance);
}

#[deprecated(note = "Use `opencc_delete` instead")]
/// C API function `opencc_free`.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_free(instance: *mut OpenCC) {
    free_opencc_instance(instance);
}

#[inline]
fn free_opencc_instance(instance: *mut OpenCC) {
    if !instance.is_null() {
        unsafe {
            let _ = Box::from_raw(instance);
        }
    }
}

// ============================================================================
// Instance options
// ============================================================================

/// C API function `opencc_get_parallel`.
///
/// Returns `false` if `instance` is NULL.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_get_parallel(instance: *const OpenCC) -> bool {
    match unsafe { instance.as_ref() } {
        Some(opencc) => opencc.get_parallel(),
        None => false,
    }
}

/// C API function `opencc_set_parallel`.
///
/// Does nothing if `instance` is NULL.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_set_parallel(instance: *mut OpenCC, is_parallel: bool) {
    if let Some(opencc) = unsafe { instance.as_mut() } {
        opencc.set_parallel(is_parallel);
    }
}

// ============================================================================
// Conversion API
// ============================================================================

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

/// C API function `opencc_convert_cfg`.
///
/// Available since **v0.8.4**.
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

/// C API function `opencc_convert_cfg_mem`.
///
/// Available since **v0.8.4**.
///
/// Writes the converted UTF-8 output into a caller-provided buffer.
///
/// This legacy memory API accepts a NUL-terminated UTF-8 input string.
/// For high-throughput interop scenarios that already know the input byte length,
/// prefer [`opencc_convert_cfg_mem_len`], which avoids the native input scan.
///
/// Contract:
/// - `out_required` must be non-NULL
/// - `*out_required` is always set to the required size in bytes, including trailing `\0`
/// - If `out_buf` is NULL or `out_cap == 0`, this acts as a size-query
/// - Returns `true` on success, `false` on failure
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
    if out_required.is_null() {
        return false;
    }

    if instance.is_null() || input.is_null() {
        return fail_with_buffer_msg(
            "Invalid argument: instance or input is NULL",
            out_buf,
            out_cap,
            out_required,
        );
    }

    let input_str = match unsafe { CStr::from_ptr(input) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            return fail_with_buffer_msg("Invalid UTF-8 input", out_buf, out_cap, out_required);
        }
    };

    convert_cfg_mem_core(
        instance,
        input_str,
        config,
        punctuation,
        out_buf,
        out_cap,
        out_required,
    )
}

/// C API function `opencc_convert_cfg_mem_len`.
///
/// Available since **v0.9.1.1**.
///
/// Writes the converted UTF-8 output into a caller-provided buffer using an
/// explicit input byte length.
///
/// This is the preferred allocation-minimizing buffer API for interop callers
/// that already have UTF-8 bytes and know the exact input length. Unlike
/// [`opencc_convert_cfg_mem`], the input does not need to be NUL-terminated.
///
/// Contract:
/// - `out_required` must be non-NULL
/// - `input` must point to exactly `input_len` bytes of valid UTF-8
/// - `*out_required` is always set to the required size in bytes, including trailing `\0`
/// - If `out_buf` is NULL or `out_cap == 0`, this acts as a size-query
/// - Returns `true` on success, `false` on failure
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_convert_cfg_mem_len(
    instance: *const OpenCC,
    input: *const u8,
    input_len: usize,
    config: u32,
    punctuation: bool,
    out_buf: *mut c_char,
    out_cap: usize,
    out_required: *mut usize,
) -> bool {
    if out_required.is_null() {
        return false;
    }

    if instance.is_null() || input.is_null() {
        return fail_with_buffer_msg(
            "Invalid argument: instance or input is NULL",
            out_buf,
            out_cap,
            out_required,
        );
    }

    let input_bytes = unsafe { std::slice::from_raw_parts(input, input_len) };
    let input_str = match std::str::from_utf8(input_bytes) {
        Ok(s) => s,
        Err(_) => {
            return fail_with_buffer_msg("Invalid UTF-8 input", out_buf, out_cap, out_required);
        }
    };

    convert_cfg_mem_core(
        instance,
        input_str,
        config,
        punctuation,
        out_buf,
        out_cap,
        out_required,
    )
}

/// C API function `opencc_convert_len`.
///
/// Converts a UTF-8 input buffer with explicit byte length using a string config.
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
    if config.is_null() {
        OpenCC::set_last_error("Invalid argument: config is NULL");
        return ptr::null_mut();
    }

    let config_str = match unsafe { CStr::from_ptr(config) }.to_str() {
        Ok(s) => s,
        Err(_) => return fail_c_string("Invalid UTF-8 config string"),
    };

    let cfg = match OpenccConfig::try_from(config_str) {
        Ok(c) => c,
        Err(_) => return fail_c_string(&format!("Invalid config: {}", config_str)),
    };

    convert_len_core(instance, input, input_len, cfg, punctuation)
}

/// C API function `opencc_convert_cfg_len`.
///
/// Converts a UTF-8 input buffer with explicit byte length using a numeric config.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_convert_cfg_len(
    instance: *const OpenCC,
    input: *const c_char,
    input_len: usize,
    config: u32,
    punctuation: bool,
) -> *mut c_char {
    let cfg = match OpenccConfig::from_ffi(config) {
        Some(c) => c,
        None => return fail_c_string(&format!("Invalid config: {}", config)),
    };

    convert_len_core(instance, input, input_len, cfg, punctuation)
}

// ============================================================================
// Other API
// ============================================================================

/// C API function `opencc_zho_check`.
///
/// Returns `-1` if `instance` or `input` is NULL, or if the input is invalid UTF-8.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_zho_check(instance: *const OpenCC, input: *const c_char) -> i32 {
    if instance.is_null() || input.is_null() {
        return -1;
    }

    let opencc = unsafe { &*instance };
    let input_str = match unsafe { CStr::from_ptr(input) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    opencc.zho_check(input_str)
}

// ============================================================================
// Error API
// ============================================================================

/// C API function `opencc_last_error`.
///
/// Contract:
/// - Always returns a heap-allocated NUL-terminated string
/// - Caller must free it using `opencc_error_free()`
/// - Returns `"No error"` when there is no error
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
#[no_mangle]
pub extern "C" fn opencc_last_error() -> *mut c_char {
    let msg = match OpenCC::get_last_error() {
        Some(err) if !err.is_empty() => err,
        _ => "No error".to_string(),
    };

    CString::new(msg)
        .unwrap_or_else(|_| CString::new("No error").expect("static has no NUL"))
        .into_raw()
}

/// C API function `opencc_clear_last_error`.
///
/// Available since **v0.8.4**.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
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
    free_c_string(ptr);
}

// ============================================================================
// String memory API
// ============================================================================

/// C API function `opencc_string_free`.
///
/// Frees a string returned by conversion functions such as `opencc_convert()`
/// or `opencc_convert_cfg()`.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_string_free(ptr: *mut c_char) {
    free_c_string(ptr);
}

#[inline]
fn free_c_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

// ============================================================================
// Config enum FFI helpers
// ============================================================================

/// C API function `opencc_config_name_to_id`.
///
/// Available since **v0.8.4**.
///
/// Returns `1` on success and writes the numeric config id to `out_id`.
/// Returns `0` on failure.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
/// Pointers passed from C must be valid for the duration of the call.
#[no_mangle]
pub extern "C" fn opencc_config_name_to_id(name_utf8: *const c_char, out_id: *mut u32) -> u8 {
    if name_utf8.is_null() || out_id.is_null() {
        return 0;
    }

    let bytes = unsafe { CStr::from_ptr(name_utf8) }.to_bytes();

    match parse_ascii_config_name(bytes) {
        Some(cfg) => {
            unsafe {
                *out_id = cfg.to_ffi();
            }
            1
        }
        None => 0,
    }
}

/// C API function `opencc_config_id_to_name`.
///
/// Available since **v0.8.4**.
///
/// Returns a pointer to a static NUL-terminated UTF-8 string,
/// or NULL if the id is invalid.
///
/// # Safety
/// This function follows the OpenCC-FMMSEG C ABI contract.
#[no_mangle]
pub extern "C" fn opencc_config_id_to_name(id: u32) -> *const c_char {
    match OpenccConfig::from_ffi(id) {
        Some(cfg) => config_to_c_name(cfg),
        None => ptr::null(),
    }
}

// ============================================================================
// Private helpers
// ============================================================================

#[inline]
fn make_c_string_or_fallback(s: &str, fallback: &'static str) -> *mut c_char {
    CString::new(s)
        .unwrap_or_else(|_| CString::new(fallback).expect("static has no NUL"))
        .into_raw()
}

#[inline]
fn fail_c_string(msg: &str) -> *mut c_char {
    OpenCC::set_last_error(msg);
    make_c_string_or_fallback(msg, "Error")
}

#[inline]
fn decode_utf8<'a>(ptr_: *const c_char, what: &'static str) -> Result<&'a str, *mut c_char> {
    let s = unsafe { CStr::from_ptr(ptr_) };
    match s.to_str() {
        Ok(v) => Ok(v),
        Err(_) => Err(fail_c_string(match what {
            "input" => "Invalid UTF-8 input",
            "config" => "Invalid UTF-8 config string",
            _ => "Invalid UTF-8 string",
        })),
    }
}

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
        Err(msg) => return fail_c_string(&msg),
    };

    let result = opencc.convert_with_config(input_str, cfg, punctuation);

    match CString::new(result) {
        Ok(cstr) => {
            OpenCC::clear_last_error();
            cstr.into_raw()
        }
        Err(_) => fail_c_string("Output contains NUL byte"),
    }
}

#[inline]
fn convert_len_core(
    instance: *const OpenCC,
    input: *const c_char,
    input_len: usize,
    cfg: OpenccConfig,
    punctuation: bool,
) -> *mut c_char {
    if instance.is_null() || input.is_null() {
        OpenCC::set_last_error("Invalid argument: instance/input is NULL");
        return ptr::null_mut();
    }

    let opencc = unsafe { &*instance };
    let input_bytes = unsafe { std::slice::from_raw_parts(input as *const u8, input_len) };

    let input_str = match std::str::from_utf8(input_bytes) {
        Ok(s) => s,
        Err(_) => return fail_c_string("Invalid UTF-8 input"),
    };

    let result = opencc.convert_with_config(input_str, cfg, punctuation);

    match CString::new(result) {
        Ok(cstr) => {
            OpenCC::clear_last_error();
            cstr.into_raw()
        }
        Err(_) => fail_c_string("Output contains NUL byte"),
    }
}

#[inline]
fn convert_cfg_mem_core(
    instance: *const OpenCC,
    input_str: &str,
    config: u32,
    punctuation: bool,
    out_buf: *mut c_char,
    out_cap: usize,
    out_required: *mut usize,
) -> bool {
    let cfg = match OpenccConfig::from_ffi(config) {
        Some(c) => c,
        None => {
            let msg = format!("Invalid config: {}", config);
            return fail_with_buffer_msg(&msg, out_buf, out_cap, out_required);
        }
    };

    let opencc = unsafe { &*instance };
    let output = opencc.convert_with_config(input_str, cfg, punctuation);

    if output.as_bytes().contains(&0) {
        return fail_with_buffer_msg("Output contains NUL byte", out_buf, out_cap, out_required);
    }

    match unsafe { write_output_bytes(output.as_bytes(), out_buf, out_cap, out_required) } {
        Ok(()) => {
            OpenCC::clear_last_error();
            true
        }
        Err(()) => {
            OpenCC::set_last_error("Output buffer too small");
            false
        }
    }
}

#[inline]
unsafe fn write_output_bytes(
    bytes: &[u8],
    out_buf: *mut c_char,
    out_cap: usize,
    out_required: *mut usize,
) -> Result<(), ()> {
    let required = bytes.len() + 1;
    *out_required = required;

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

#[inline]
fn fail_with_buffer_msg(
    msg: &str,
    out_buf: *mut c_char,
    out_cap: usize,
    out_required: *mut usize,
) -> bool {
    OpenCC::set_last_error(msg);
    let bytes = msg.as_bytes();
    let safe_bytes = if bytes.contains(&0) { b"Error" } else { bytes };

    let write_ok =
        unsafe { write_output_bytes(safe_bytes, out_buf, out_cap, out_required).is_ok() };

    if !write_ok && !(out_buf.is_null() || out_cap == 0) {
        OpenCC::set_last_error("Output buffer too small");
    }

    false
}

#[inline]
fn parse_ascii_config_name(bytes: &[u8]) -> Option<OpenccConfig> {
    if !bytes.iter().all(|&b| b.is_ascii_alphanumeric()) {
        return None;
    }

    if eq_ascii_ci(bytes, b"s2t") {
        return Some(OpenccConfig::S2t);
    }
    if eq_ascii_ci(bytes, b"s2tw") {
        return Some(OpenccConfig::S2tw);
    }
    if eq_ascii_ci(bytes, b"s2twp") {
        return Some(OpenccConfig::S2twp);
    }
    if eq_ascii_ci(bytes, b"s2hk") {
        return Some(OpenccConfig::S2hk);
    }
    if eq_ascii_ci(bytes, b"t2s") {
        return Some(OpenccConfig::T2s);
    }
    if eq_ascii_ci(bytes, b"t2tw") {
        return Some(OpenccConfig::T2tw);
    }
    if eq_ascii_ci(bytes, b"t2twp") {
        return Some(OpenccConfig::T2twp);
    }
    if eq_ascii_ci(bytes, b"t2hk") {
        return Some(OpenccConfig::T2hk);
    }
    if eq_ascii_ci(bytes, b"tw2s") {
        return Some(OpenccConfig::Tw2s);
    }
    if eq_ascii_ci(bytes, b"tw2sp") {
        return Some(OpenccConfig::Tw2sp);
    }
    if eq_ascii_ci(bytes, b"tw2t") {
        return Some(OpenccConfig::Tw2t);
    }
    if eq_ascii_ci(bytes, b"tw2tp") {
        return Some(OpenccConfig::Tw2tp);
    }
    if eq_ascii_ci(bytes, b"hk2s") {
        return Some(OpenccConfig::Hk2s);
    }
    if eq_ascii_ci(bytes, b"hk2t") {
        return Some(OpenccConfig::Hk2t);
    }
    if eq_ascii_ci(bytes, b"jp2t") {
        return Some(OpenccConfig::Jp2t);
    }
    if eq_ascii_ci(bytes, b"t2jp") {
        return Some(OpenccConfig::T2jp);
    }

    None
}

#[inline]
fn eq_ascii_ci(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b.iter())
            .all(|(&x, &y)| x.to_ascii_lowercase() == y)
}

#[inline]
fn config_to_c_name(cfg: OpenccConfig) -> *const c_char {
    match cfg {
        OpenccConfig::S2t => b"s2t\0".as_ptr() as *const c_char,
        OpenccConfig::S2tw => b"s2tw\0".as_ptr() as *const c_char,
        OpenccConfig::S2twp => b"s2twp\0".as_ptr() as *const c_char,
        OpenccConfig::S2hk => b"s2hk\0".as_ptr() as *const c_char,
        OpenccConfig::T2s => b"t2s\0".as_ptr() as *const c_char,
        OpenccConfig::T2tw => b"t2tw\0".as_ptr() as *const c_char,
        OpenccConfig::T2twp => b"t2twp\0".as_ptr() as *const c_char,
        OpenccConfig::T2hk => b"t2hk\0".as_ptr() as *const c_char,
        OpenccConfig::Tw2s => b"tw2s\0".as_ptr() as *const c_char,
        OpenccConfig::Tw2sp => b"tw2sp\0".as_ptr() as *const c_char,
        OpenccConfig::Tw2t => b"tw2t\0".as_ptr() as *const c_char,
        OpenccConfig::Tw2tp => b"tw2tp\0".as_ptr() as *const c_char,
        OpenccConfig::Hk2s => b"hk2s\0".as_ptr() as *const c_char,
        OpenccConfig::Hk2t => b"hk2t\0".as_ptr() as *const c_char,
        OpenccConfig::Jp2t => b"jp2t\0".as_ptr() as *const c_char,
        OpenccConfig::T2jp => b"t2jp\0".as_ptr() as *const c_char,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[inline]
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
        assert_eq!(opencc_abi_number(), 1);
    }

    #[test]
    fn opencc_version_string_is_valid_utf8_and_non_empty() {
        let ptr = opencc_version_string();
        assert!(!ptr.is_null());

        let ver = unsafe { CStr::from_ptr(ptr) }
            .to_str()
            .expect("version string must be valid UTF-8");

        assert!(!ver.is_empty());
    }

    #[test]
    fn test_opencc_zho_check() {
        let opencc = OpenCC::new();
        let input = CString::new("你好，世界，欢迎").unwrap();

        let result = opencc_zho_check(&opencc as *const OpenCC, input.as_ptr());
        assert_eq!(result, 2);
    }

    #[test]
    fn test_opencc_convert() {
        let opencc = OpenCC::new();
        let input = CString::new("意大利罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。").unwrap();
        let config = CString::new("s2twp").unwrap();

        let result_ptr = opencc_convert(
            &opencc as *const OpenCC,
            input.as_ptr(),
            config.as_ptr(),
            true,
        );
        let result = unsafe { CStr::from_ptr(result_ptr) }
            .to_string_lossy()
            .into_owned();
        opencc_string_free(result_ptr);

        assert_eq!(
            result,
            "義大利羅浮宮裡收藏的「蒙娜麗莎的微笑」畫像是曠世之作。"
        );
    }

    #[test]
    fn test_opencc_convert_cfg() {
        let opencc = OpenCC::new();
        let input = CString::new("意大利罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。").unwrap();

        let result_ptr = opencc_convert_cfg(
            &opencc as *const OpenCC,
            input.as_ptr(),
            OpenccConfig::S2twp.to_ffi(),
            true,
        );

        let result = unsafe { CStr::from_ptr(result_ptr) }
            .to_string_lossy()
            .into_owned();
        opencc_string_free(result_ptr);

        assert_eq!(
            result,
            "義大利羅浮宮裡收藏的「蒙娜麗莎的微笑」畫像是曠世之作。"
        );
    }

    #[test]
    fn test_opencc_convert_len() {
        let opencc = OpenCC::new();
        let input_str = "意大利罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";
        let input = CString::new(input_str).unwrap();
        let config = CString::new("s2twp").unwrap();

        let result_ptr = opencc_convert_len(
            &opencc as *const OpenCC,
            input.as_ptr(),
            input_str.len(), // explicit length (no '\0' scan)
            config.as_ptr(),
            true,
        );

        let result = unsafe { CStr::from_ptr(result_ptr) }
            .to_string_lossy()
            .into_owned();
        opencc_string_free(result_ptr);

        assert_eq!(
            result,
            "義大利羅浮宮裡收藏的「蒙娜麗莎的微笑」畫像是曠世之作。"
        );
    }

    #[test]
    fn test_opencc_convert_cfg_len() {
        let opencc = OpenCC::new();
        let input_str = "意大利罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";
        let input = CString::new(input_str).unwrap();

        let result_ptr = opencc_convert_cfg_len(
            &opencc as *const OpenCC,
            input.as_ptr(),
            input_str.len(), // explicit length
            OpenccConfig::S2twp.to_ffi(),
            true,
        );

        let result = unsafe { CStr::from_ptr(result_ptr) }
            .to_string_lossy()
            .into_owned();
        opencc_string_free(result_ptr);

        assert_eq!(
            result,
            "義大利羅浮宮裡收藏的「蒙娜麗莎的微笑」畫像是曠世之作。"
        );
    }

    #[test]
    fn test_opencc_convert_len_no_null() {
        let opencc = OpenCC::new();

        let input_str = "意大利罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";
        let input_bytes = input_str.as_bytes(); // NOT null-terminated

        let config = CString::new("s2twp").unwrap();

        let result_ptr = opencc_convert_len(
            &opencc as *const OpenCC,
            input_bytes.as_ptr() as *const c_char,
            input_bytes.len(), // exact length, no '\0'
            config.as_ptr(),
            true,
        );

        let result = unsafe { CStr::from_ptr(result_ptr) }
            .to_string_lossy()
            .into_owned();
        opencc_string_free(result_ptr);

        assert_eq!(
            result,
            "義大利羅浮宮裡收藏的「蒙娜麗莎的微笑」畫像是曠世之作。"
        );
    }

    #[test]
    fn test_opencc_convert_cfg_len_no_null() {
        let opencc = OpenCC::new();

        let input_str = "意大利罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";
        let input_bytes = input_str.as_bytes(); // raw buffer

        let result_ptr = opencc_convert_cfg_len(
            &opencc as *const OpenCC,
            input_bytes.as_ptr() as *const c_char,
            input_bytes.len(),
            OpenccConfig::S2twp.to_ffi(),
            true,
        );

        let result = unsafe { CStr::from_ptr(result_ptr) }
            .to_string_lossy()
            .into_owned();
        opencc_string_free(result_ptr);

        assert_eq!(
            result,
            "義大利羅浮宮裡收藏的「蒙娜麗莎的微笑」畫像是曠世之作。"
        );
    }

    #[test]
    fn test_opencc_convert_cfg_mem_len_size_query_and_write() {
        let opencc = OpenCC::new();
        let input = "意大利罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";
        let input_bytes = input.as_bytes();
        let mut required = 0usize;

        let ok_query = opencc_convert_cfg_mem_len(
            &opencc as *const OpenCC,
            input_bytes.as_ptr(),
            input_bytes.len(),
            OpenccConfig::S2twp.to_ffi(),
            true,
            ptr::null_mut(),
            0,
            &mut required,
        );

        assert!(ok_query);
        assert!(required > 0);

        let mut out = vec![0u8; required];
        let ok_write = opencc_convert_cfg_mem_len(
            &opencc as *const OpenCC,
            input_bytes.as_ptr(),
            input_bytes.len(),
            OpenccConfig::S2twp.to_ffi(),
            true,
            out.as_mut_ptr() as *mut c_char,
            out.len(),
            &mut required,
        );

        assert!(ok_write);
        assert_eq!(
            &out[..required - 1],
            "義大利羅浮宮裡收藏的「蒙娜麗莎的微笑」畫像是曠世之作。".as_bytes()
        );
        assert_eq!(out[required - 1], 0);
    }

    #[test]
    fn test_opencc_convert_cfg_mem_len_invalid_utf8() {
        let opencc = OpenCC::new();
        let input_bytes = [0xFFu8, 0x00u8];
        let mut required = 0usize;

        let ok = opencc_convert_cfg_mem_len(
            &opencc as *const OpenCC,
            input_bytes.as_ptr(),
            input_bytes.len(),
            OpenccConfig::S2t.to_ffi(),
            false,
            ptr::null_mut(),
            0,
            &mut required,
        );

        assert!(!ok);
        assert!(read_and_free(opencc_last_error()).contains("Invalid UTF-8 input"));
    }

    #[test]
    fn test_opencc_invalid_config() {
        OpenCC::set_last_error("");

        let opencc = OpenCC::new();
        let input = CString::new("你好，世界，欢迎！").unwrap();
        let config = CString::new("s2s").unwrap();

        let result_ptr = opencc_convert(
            &opencc as *const OpenCC,
            input.as_ptr(),
            config.as_ptr(),
            false,
        );
        let result = unsafe { CStr::from_ptr(result_ptr) }
            .to_string_lossy()
            .into_owned();
        opencc_string_free(result_ptr);

        assert_eq!(result, "Invalid config: s2s");
        assert!(OpenCC::get_last_error().unwrap().contains("Invalid"));

        OpenCC::set_last_error("");
    }

    #[test]
    fn test_opencc_last_error_default() {
        OpenCC::set_last_error("");
        let msg = read_and_free(opencc_last_error());
        assert_eq!(msg, "No error");
    }

    #[test]
    fn test_opencc_last_error_roundtrip() {
        OpenCC::set_last_error("");
        let _ = OpenCC::from_cbor("test.json");

        let expected = OpenCC::get_last_error().unwrap_or_else(|| "No error".to_string());
        let actual = read_and_free(opencc_last_error());

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_name_to_id_success() {
        let name = CString::new("s2t").unwrap();
        let mut out_id = 0u32;

        let ok = opencc_config_name_to_id(name.as_ptr(), &mut out_id);

        assert_eq!(ok, 1);
        assert_eq!(out_id, 1);
    }

    #[test]
    fn test_id_to_name_success() {
        let ptr = opencc_config_id_to_name(1);
        assert!(!ptr.is_null());

        let cstr = unsafe { CStr::from_ptr(ptr) };
        assert_eq!(cstr.to_str().unwrap(), "s2t");
    }

    #[test]
    fn test_config_helpers_invalid_inputs() {
        let name = CString::new("invalid").unwrap();
        let mut out_id = 123u32;

        let ok = opencc_config_name_to_id(name.as_ptr(), &mut out_id);
        assert_eq!(ok, 0);
        assert_eq!(out_id, 123);

        let ptr = opencc_config_id_to_name(999);
        assert!(ptr.is_null());

        let ok = opencc_config_name_to_id(ptr::null(), ptr::null_mut());
        assert_eq!(ok, 0);
    }
}
