import ctypes
from ctypes import wintypes
import test_find_device as tfd

hid = ctypes.windll.hid
kernel32 = ctypes.windll.kernel32
setupapi = ctypes.windll.setupapi

hid_guid = tfd.GUID()
hid.HidD_GetHidGuid(ctypes.byref(hid_guid))
dev_info = setupapi.SetupDiGetClassDevsW(ctypes.byref(hid_guid), None, None, 2 | 16)

device_interface_data = tfd.SP_DEVICE_INTERFACE_DATA()
device_interface_data.cbSize = ctypes.sizeof(tfd.SP_DEVICE_INTERFACE_DATA)
index = 0

while setupapi.SetupDiEnumDeviceInterfaces(dev_info, None, ctypes.byref(hid_guid), index, ctypes.byref(device_interface_data)):
    required_size = wintypes.DWORD()
    setupapi.SetupDiGetDeviceInterfaceDetailW(dev_info, ctypes.byref(device_interface_data), None, 0, ctypes.byref(required_size), None)
    detail_size = required_size.value
    buf = (ctypes.c_byte * detail_size)()
    struct_cbSize = 8 if ctypes.sizeof(ctypes.c_void_p) == 8 else (4 + ctypes.sizeof(ctypes.c_wchar))
    ctypes.cast(ctypes.byref(buf), ctypes.POINTER(wintypes.DWORD))[0] = struct_cbSize

    if setupapi.SetupDiGetDeviceInterfaceDetailW(dev_info, ctypes.byref(device_interface_data), ctypes.byref(buf), detail_size, None, None):
        dev_path = ctypes.wstring_at(ctypes.addressof(buf) + 4)
        if "0b05" in dev_path.lower():
            h = kernel32.CreateFileW(dev_path, 0, 1 | 2, None, 3, 0, None)
            if h != -1 and h != 0:
                preparsed = ctypes.c_void_p()
                caps = tfd.HIDP_CAPS()
                if hid.HidD_GetPreparsedData(h, ctypes.byref(preparsed)):
                    hid.HidP_GetCaps(preparsed, ctypes.byref(caps))
                    hid.HidD_FreePreparsedData(preparsed)
                    print(f"Path: {dev_path[:65]}... | Page: {hex(caps.UsagePage)} | Usage: {hex(caps.Usage)} | FeatLen: {caps.FeatureReportByteLength}")
                else:
                    print(f"Path: {dev_path[:65]}... | HidD_GetPreparsedData failed: {kernel32.GetLastError()}")
                kernel32.CloseHandle(h)
            else:
                print(f"Path: {dev_path[:65]}... | CreateFile failed: {kernel32.GetLastError()}")
    index += 1

setupapi.SetupDiDestroyDeviceInfoList(dev_info)
