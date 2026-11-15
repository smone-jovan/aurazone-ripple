//! Win32 Direct HID device implementation for ASUS ROG Aura Col05 controller.

use std::ffi::c_void;
use std::time::Duration;
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
    SetupDiGetDeviceInterfaceDetailW, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT,
    HDEVINFO, SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W,
};
use windows_sys::Win32::Devices::HumanInterfaceDevice::{
    HidD_FreePreparsedData, HidD_GetAttributes, HidD_GetHidGuid, HidD_GetPreparsedData,
    HidD_SetFeature, HidP_GetCaps, HIDD_ATTRIBUTES, HIDP_CAPS,
};

use super::protocol::{encode_direct_frame, encode_handshake};
use super::{AuraDevice, AuraError};

const GENERIC_READ: u32 = 0x8000_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;

const ASUS_VID: u16 = 0x0B05;
const ASUS_PID: u16 = 0x19B6;
const AURA_USAGE_PAGE: u16 = 0xFF31;
const AURA_COL05_USAGE: u16 = 0x0079;
const EXPECTED_FEATURE_LEN: u16 = 64;

/// Known physical device path on ASUS ROG Strix G513RC
pub const KNOWN_COL05_DEVICE_PATH: &str =
    r"\\?\HID#VID_0B05&PID_19B6&Col05#7&2d1c04e0&0&0004#{4d1e55b2-f16f-11cf-88cb-001111000030}";

/// Win32 direct hardware device handle for ASUS ROG Aura controller.
pub struct Win32AuraDevice {
    handle: HANDLE,
    path: String,
}

// Raw Windows HANDLE is !Send and !Sync by default.
// Safe because methods require &mut self or synchronize access.
unsafe impl Send for Win32AuraDevice {}
unsafe impl Sync for Win32AuraDevice {}

impl Win32AuraDevice {
    /// Discovers and opens the ASUS ROG Aura Col05 direct lighting device.
    /// Sends the initial handshake primer and sleeps 50 ms before returning.
    pub fn open() -> Result<Self, AuraError> {
        let path = Self::find_col05_path()?;
        let mut device = Self::open_by_path(&path)?;

        // Send initial handshake primer to switch micro-controller to direct lighting mode
        let handshake = encode_handshake();
        device.send_feature(&handshake)?;
        std::thread::sleep(Duration::from_millis(50));

        Ok(device)
    }

    /// Enumerates HID interfaces using SetupAPI to find the Col05 endpoint.
    pub fn find_col05_path() -> Result<String, AuraError> {
        unsafe {
            let mut hid_guid = std::mem::zeroed();
            HidD_GetHidGuid(&mut hid_guid);

            let hdevinfo: HDEVINFO = SetupDiGetClassDevsW(
                &hid_guid,
                std::ptr::null(),
                std::ptr::null_mut(),
                DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
            );

            if hdevinfo == 0 || hdevinfo == (INVALID_HANDLE_VALUE as isize) {
                return Err(AuraError::from(GetLastError()));
            }

            struct DevInfoGuard(HDEVINFO);
            impl Drop for DevInfoGuard {
                fn drop(&mut self) {
                    unsafe {
                        SetupDiDestroyDeviceInfoList(self.0);
                    }
                }
            }
            let _guard = DevInfoGuard(hdevinfo);

            let mut dev_interface_data: SP_DEVICE_INTERFACE_DATA = std::mem::zeroed();
            dev_interface_data.cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;

            let mut member_index = 0;
            loop {
                let res = SetupDiEnumDeviceInterfaces(
                    hdevinfo,
                    std::ptr::null(),
                    &hid_guid,
                    member_index,
                    &mut dev_interface_data,
                );

                if res == 0 {
                    let err = GetLastError();
                    if err == 259 {
                        // ERROR_NO_MORE_ITEMS
                        break;
                    }
                    member_index += 1;
                    continue;
                }

                // Pass 1: get required size
                let mut req_size: u32 = 0;
                SetupDiGetDeviceInterfaceDetailW(
                    hdevinfo,
                    &dev_interface_data,
                    std::ptr::null_mut(),
                    0,
                    &mut req_size,
                    std::ptr::null_mut(),
                );

                if req_size > 0 {
                    let mut buffer: Vec<u8> = vec![0u8; req_size as usize];
                    let detail_ptr = buffer.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;

                    // CRITICAL 64-bit alignment rule: cbSize MUST be 8 on 64-bit Windows
                    #[cfg(target_pointer_width = "64")]
                    {
                        (*detail_ptr).cbSize = 8;
                    }
                    #[cfg(target_pointer_width = "32")]
                    {
                        (*detail_ptr).cbSize = 5;
                    }

                    if SetupDiGetDeviceInterfaceDetailW(
                        hdevinfo,
                        &dev_interface_data,
                        detail_ptr,
                        req_size,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    ) != 0
                    {
                        let path_ptr = std::ptr::addr_of!((*detail_ptr).DevicePath) as *const u16;
                        let mut len = 0;
                        while *path_ptr.add(len) != 0 {
                            len += 1;
                        }
                        let path_slice = std::slice::from_raw_parts(path_ptr, len);
                        let path_str = String::from_utf16_lossy(path_slice);
                        let path_lower = path_str.to_lowercase();

                        // Fast string filter
                        if path_lower.contains("0b05")
                            && path_lower.contains("19b6")
                            && path_lower.contains("col05")
                        {
                            // Authoritative capability verification
                            if Self::verify_endpoint_caps(&path_str) {
                                return Ok(path_str);
                            }
                        }
                    }
                }

                member_index += 1;
            }

            // Fallback: Check known ROG Strix hardware interface path
            if Self::verify_endpoint_caps(KNOWN_COL05_DEVICE_PATH) {
                return Ok(KNOWN_COL05_DEVICE_PATH.to_string());
            }

            Err(AuraError::DeviceNotFound)
        }
    }

    /// Verifies VID, PID, UsagePage, and Usage via HID capabilities query.
    pub fn verify_endpoint_caps(path: &str) -> bool {
        let wide_path: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let handle = CreateFileW(
                wide_path.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            );

            if handle.is_null() || handle == INVALID_HANDLE_VALUE {
                return false;
            }

            let mut attr: HIDD_ATTRIBUTES = std::mem::zeroed();
            attr.Size = std::mem::size_of::<HIDD_ATTRIBUTES>() as u32;
            if HidD_GetAttributes(handle, &mut attr) == 0 {
                CloseHandle(handle);
                return false;
            }

            if attr.VendorID != ASUS_VID || attr.ProductID != ASUS_PID {
                CloseHandle(handle);
                return false;
            }

            let mut preparsed: isize = 0;
            if HidD_GetPreparsedData(handle, &mut preparsed) == 0 {
                CloseHandle(handle);
                return false;
            }

            let mut caps: HIDP_CAPS = std::mem::zeroed();
            let caps_res = HidP_GetCaps(preparsed, &mut caps);
            HidD_FreePreparsedData(preparsed);
            CloseHandle(handle);

            // UsagePage 0xFF31, Usage 0x0079, FeatureReportByteLength 64
            caps_res != 0
                && caps.UsagePage == AURA_USAGE_PAGE
                && caps.Usage == AURA_COL05_USAGE
                && caps.FeatureReportByteLength == EXPECTED_FEATURE_LEN
        }
    }

    /// Opens handle to device path with shared read/write mode.
    pub fn open_by_path(path: &str) -> Result<Self, AuraError> {
        let wide_path: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let handle = CreateFileW(
                wide_path.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            );

            if handle.is_null() || handle == INVALID_HANDLE_VALUE {
                return Err(AuraError::from(GetLastError()));
            }

            Ok(Self {
                handle,
                path: path.to_string(),
            })
        }
    }

    /// Returns the active Windows device interface path.
    pub fn device_path(&self) -> &str {
        &self.path
    }
}

impl AuraDevice for Win32AuraDevice {
    fn send_feature(&mut self, report: &[u8; 64]) -> Result<(), AuraError> {
        unsafe {
            let ok = HidD_SetFeature(self.handle, report.as_ptr() as *const c_void, 64);

            if ok == 0 {
                let err = GetLastError();
                return Err(AuraError::from(err));
            }

            Ok(())
        }
    }

    fn apply_zones(&mut self, zone_colors: &[[u8; 3]; 8]) -> Result<(), AuraError> {
        let frame = encode_direct_frame(zone_colors);
        self.send_feature(&frame)
    }

    fn set_static_uniform(&mut self, color: [u8; 3]) -> Result<(), AuraError> {
        let colors = [color; 8];
        self.apply_zones(&colors)
    }
}

impl Drop for Win32AuraDevice {
    fn drop(&mut self) {
        if !self.handle.is_null() && self.handle != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.handle);
            }
            self.handle = std::ptr::null_mut();
        }
    }
}
