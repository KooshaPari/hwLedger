//! Dump raw PerformanceStatistics keys + HID temperature sensor names for
//! debugging chip-specific differences. Not used in production.
#[cfg(target_os = "macos")]
fn main() {
    use core_foundation::array::{CFArray, CFArrayRef};
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
    use core_foundation::number::CFNumber;
    use core_foundation::string::{CFString, CFStringRef};
    use core_foundation_sys::base::{kCFAllocatorDefault, CFRelease};
    use io_kit_sys::{
        kIOMasterPortDefault, IOIteratorNext, IOObjectRelease, IORegistryEntryCreateCFProperties,
        IOServiceGetMatchingServices, IOServiceMatching,
    };
    use std::ffi::CString;

    unsafe {
        let cname = CString::new("AGXAccelerator").unwrap();
        let matching = IOServiceMatching(cname.as_ptr());
        let mut iter = 0;
        IOServiceGetMatchingServices(kIOMasterPortDefault, matching, &mut iter);
        let svc = IOIteratorNext(iter);
        IOObjectRelease(iter);

        let mut props = std::ptr::null_mut();
        IORegistryEntryCreateCFProperties(svc, &mut props, kCFAllocatorDefault, 0);
        IOObjectRelease(svc);

        let top: CFDictionary<CFString, CFType> =
            CFDictionary::wrap_under_create_rule(props as CFDictionaryRef);
        let perf_key = CFString::from_static_string("PerformanceStatistics");
        let perf = top.find(&perf_key).unwrap();
        let perf_dict: CFDictionary<CFString, CFType> =
            CFDictionary::wrap_under_get_rule(perf.as_CFTypeRef() as CFDictionaryRef);
        let (keys, values) = perf_dict.get_keys_and_values();
        println!("== PerformanceStatistics ({}) ==", keys.len());
        for (k, v) in keys.iter().zip(values.iter()) {
            let ks = CFString::wrap_under_get_rule(*k as CFStringRef).to_string();
            let vt: CFType = CFType::wrap_under_get_rule(*v);
            if vt.instance_of::<CFNumber>() {
                let n: CFNumber =
                    CFNumber::wrap_under_get_rule(*v as core_foundation_sys::number::CFNumberRef);
                println!("  {:<40} = {:?}", ks, n.to_f64().or(n.to_i64().map(|i| i as f64)));
            } else {
                println!("  {:<40} = <non-number>", ks);
            }
        }

        // HID sensors
        #[link(name = "IOKit", kind = "framework")]
        extern "C" {
            fn IOHIDEventSystemClientCreate(
                a: core_foundation_sys::base::CFAllocatorRef,
            ) -> *mut std::ffi::c_void;
            fn IOHIDEventSystemClientSetMatching(
                c: *mut std::ffi::c_void,
                m: CFDictionaryRef,
            ) -> i32;
            fn IOHIDEventSystemClientCopyServices(c: *mut std::ffi::c_void) -> CFArrayRef;
            fn IOHIDServiceClientCopyProperty(
                s: *mut std::ffi::c_void,
                k: CFStringRef,
            ) -> *const std::ffi::c_void;
            fn IOHIDServiceClientCopyEvent(
                s: *mut std::ffi::c_void,
                t: i64,
                o: i32,
                to: i64,
            ) -> *const std::ffi::c_void;
            fn IOHIDEventGetFloatValue(e: *const std::ffi::c_void, f: i32) -> f64;
        }
        let client = IOHIDEventSystemClientCreate(kCFAllocatorDefault);
        let page_key = CFString::from_static_string("PrimaryUsagePage");
        let usage_key = CFString::from_static_string("PrimaryUsage");
        let page_val = CFNumber::from(0xFF00_i32);
        let usage_val = CFNumber::from(0x0005_i32);
        let matching = CFDictionary::from_CFType_pairs(&[
            (page_key.as_CFType(), page_val.as_CFType()),
            (usage_key.as_CFType(), usage_val.as_CFType()),
        ]);
        IOHIDEventSystemClientSetMatching(client, matching.as_concrete_TypeRef());
        let svcs = IOHIDEventSystemClientCopyServices(client);
        let svcs: CFArray<CFType> = CFArray::wrap_under_create_rule(svcs);
        println!("\n== HID temperature sensors ({}) ==", svcs.len());
        for i in 0..svcs.len() {
            let s = svcs.get(i).unwrap();
            let sp = s.as_CFTypeRef() as *mut std::ffi::c_void;
            let name_key = CFString::from_static_string("Product");
            let np = IOHIDServiceClientCopyProperty(sp, name_key.as_concrete_TypeRef());
            let name = if np.is_null() {
                "<unknown>".to_string()
            } else {
                let cf: CFString = CFString::wrap_under_create_rule(np as CFStringRef);
                cf.to_string()
            };
            let ev = IOHIDServiceClientCopyEvent(sp, 15, 0, 0);
            let t = if ev.is_null() {
                f64::NAN
            } else {
                let v = IOHIDEventGetFloatValue(ev, 15i32 << 16);
                CFRelease(ev as _);
                v
            };
            println!("  {:<40} = {:.2}°C", name, t);
        }
        CFRelease(client as _);
    }
}
#[cfg(not(target_os = "macos"))]
fn main() {}
