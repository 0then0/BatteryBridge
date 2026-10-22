//! Minimal IOKit/CoreFoundation read boundary. Only named public battery fields are read.
use batterybridge_core::*;
use std::{
    ffi::{CString, c_char, c_void},
    ptr,
};
type Cf = *const c_void;
#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOServiceMatching(name: *const c_char) -> Cf;
    fn IOServiceGetMatchingService(port: u32, matching: Cf) -> u32;
    fn IOObjectRelease(object: u32) -> i32;
    fn IORegistryEntryCreateCFProperty(entry: u32, key: Cf, allocator: Cf, options: u32) -> Cf;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringCreateWithCString(allocator: Cf, s: *const c_char, encoding: u32) -> Cf;
    fn CFRelease(object: Cf);
    fn CFGetTypeID(object: Cf) -> usize;
    fn CFNumberGetTypeID() -> usize;
    fn CFBooleanGetTypeID() -> usize;
    fn CFNumberGetValue(number: Cf, kind: isize, value: *mut c_void) -> bool;
    fn CFBooleanGetValue(boolean: Cf) -> bool;
}
struct Object(u32);
impl Drop for Object {
    fn drop(&mut self) {
        // SAFETY: Object owns one live IOKit reference.
        unsafe {
            IOObjectRelease(self.0);
        }
    }
}
struct Value(Cf);
impl Drop for Value {
    fn drop(&mut self) {
        // SAFETY: Value only wraps non-null owned CF references.
        unsafe {
            CFRelease(self.0);
        }
    }
}
fn service() -> Result<Object> {
    // SAFETY: static NUL-terminated service name; matching dictionary is consumed by IOKit.
    let object =
        unsafe { IOServiceGetMatchingService(0, IOServiceMatching(c"AppleSmartBattery".as_ptr())) };
    if object == 0 {
        return Err(Error::BackendUnavailable(
            "AppleSmartBattery service missing".into(),
        ));
    }
    Ok(Object(object))
}
fn property(object: &Object, name: &str) -> Option<Value> {
    let name = CString::new(name).ok()?;
    // SAFETY: CString remains alive; UTF-8 encoding constant; null allocator means default.
    let key = unsafe { CFStringCreateWithCString(ptr::null(), name.as_ptr(), 0x08000100) };
    if key.is_null() {
        return None;
    }
    let key = Value(key);
    // SAFETY: both handles are owned live references; result follows Create rule.
    let value = unsafe { IORegistryEntryCreateCFProperty(object.0, key.0, ptr::null(), 0) };
    if value.is_null() {
        None
    } else {
        Some(Value(value))
    }
}
fn number(object: &Object, name: &str) -> Option<i64> {
    let v = property(object, name)?;
    // SAFETY: non-null CF reference and type is checked before CFNumber conversion.
    unsafe {
        if CFGetTypeID(v.0) != CFNumberGetTypeID() {
            return None;
        }
        let mut out = 0i64;
        CFNumberGetValue(v.0, 4, (&mut out as *mut i64).cast()).then_some(out)
    }
}
fn boolean(object: &Object, name: &str) -> Option<bool> {
    let v = property(object, name)?;
    // SAFETY: live reference; boolean type checked before conversion.
    unsafe { (CFGetTypeID(v.0) == CFBooleanGetTypeID()).then(|| CFBooleanGetValue(v.0)) }
}
pub struct Monitor;
impl BatteryMonitor for Monitor {
    fn batteries(&self) -> Result<Vec<BatteryStatus>> {
        let obj = match service() {
            Ok(v) => v,
            Err(Error::BackendUnavailable(_)) => return Ok(vec![]),
            Err(e) => return Err(e),
        };
        let current = number(&obj, "CurrentCapacity");
        let max = number(&obj, "MaxCapacity");
        let percent = current
            .zip(max)
            .filter(|(_, m)| *m > 0)
            .map(|(v, m)| (v.saturating_mul(100) / m).clamp(0, 100) as u8);
        let health = number(&obj, "AppleRawMaxCapacity")
            .zip(number(&obj, "DesignCapacity"))
            .filter(|(_, d)| *d > 0)
            .map(|(v, d)| (v.saturating_mul(100) / d).clamp(0, 100) as u8);
        let state = if boolean(&obj, "FullyCharged") == Some(true) {
            "full"
        } else if boolean(&obj, "IsCharging") == Some(true) {
            "charging"
        } else if boolean(&obj, "ExternalConnected") == Some(false) {
            "discharging"
        } else {
            "not charging"
        };
        Ok(vec![BatteryStatus {
            id: "BAT0".into(),
            percent,
            state: state.into(),
            health_percent: health,
            on_ac: boolean(&obj, "ExternalConnected"),
            hardware_start: None,
            hardware_stop: None,
        }])
    }
}

unsafe extern "C" {
    fn sysctlbyname(
        name: *const c_char,
        old: *mut c_void,
        length: *mut usize,
        new: *mut c_void,
        new_length: usize,
    ) -> i32;
}
pub fn model() -> String {
    let mut bytes = [0u8; 256];
    let mut length = bytes.len();
    // SAFETY: fixed public sysctl name and bounded writable buffer. Null new
    // pointer makes this strictly read-only; returned length is checked below.
    let result = unsafe {
        sysctlbyname(
            c"hw.model".as_ptr(),
            bytes.as_mut_ptr().cast(),
            &mut length,
            ptr::null_mut(),
            0,
        )
    };
    if result != 0 || length > bytes.len() {
        return "unknown".into();
    }
    String::from_utf8_lossy(&bytes[..length])
        .trim_end_matches('\0')
        .to_owned()
}
