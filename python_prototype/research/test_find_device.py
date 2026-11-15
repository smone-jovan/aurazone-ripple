import ctypes
from ctypes import wintypes
import time

kernel32 = ctypes.windll.kernel32
hid = ctypes.windll.hid
setupapi = ctypes.windll.setupapi

GENERIC_READ = 0x80000000
GENERIC_WRITE = 0x40000000
FILE_SHARE_READ = 1
FILE_SHARE_WRITE = 2
OPEN_EXISTING = 3

class GUID(ctypes.Structure):
    _fields_ = [
        ("Data1", wintypes.DWORD),
        ("Data2", wintypes.WORD),
        ("Data3", wintypes.WORD),
        ("Data4", wintypes.BYTE * 8)
    ]

class SP_DEVICE_INTERFACE_DATA(ctypes.Structure):
    _fields_ = [
        ("cbSize", wintypes.DWORD),
        ("InterfaceClassGuid", GUID),
        ("Flags", wintypes.DWORD),
        ("Reserved", ctypes.c_size_t)
    ]

class HIDD_ATTRIBUTES(ctypes.Structure):
    _fields_ = [
        ("Size", wintypes.DWORD),
        ("VendorID", wintypes.WORD),
        ("ProductID", wintypes.WORD),
        ("VersionNumber", wintypes.WORD)
    ]

class HIDP_CAPS(ctypes.Structure):
    _fields_ = [
        ("Usage", wintypes.USHORT),
        ("UsagePage", wintypes.USHORT),
        ("InputReportByteLength", wintypes.USHORT),
        ("OutputReportByteLength", wintypes.USHORT),
        ("FeatureReportByteLength", wintypes.USHORT),
        ("Reserved", wintypes.USHORT * 17),
        ("NumberLinkCollectionNodes", wintypes.USHORT),
        ("NumberInputButtonCaps", wintypes.USHORT),
        ("NumberInputValueCaps", wintypes.USHORT),
        ("NumberInputDataIndices", wintypes.USHORT),
        ("NumberOutputButtonCaps", wintypes.USHORT),
        ("NumberOutputValueCaps", wintypes.USHORT),
        ("NumberOutputDataIndices", wintypes.USHORT),
        ("NumberFeatureButtonCaps", wintypes.USHORT),
        ("NumberFeatureValueCaps", wintypes.USHORT),
        ("NumberFeatureDataIndices", wintypes.USHORT),
    ]

def find_aura_col05():
    hid_guid = GUID()
    hid.HidD_GetHidGuid(ctypes.byref(hid_guid))
    dev_info = setupapi.SetupDiGetClassDevsW(
        ctypes.byref(hid_guid), None, None, 2 | 16
    )
    if dev_info == -1:
        return None

    device_interface_data = SP_DEVICE_INTERFACE_DATA()
    device_interface_data.cbSize = ctypes.sizeof(SP_DEVICE_INTERFACE_DATA)
    index = 0
    matched_path = None

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
                h = kernel32.CreateFileW(dev_path, 0, FILE_SHARE_READ | FILE_SHARE_WRITE, None, OPEN_EXISTING, 0, None)
                if h != -1 and h != 0:
                    preparsed = ctypes.c_void_p()
                    caps = HIDP_CAPS()
                    if hid.HidD_GetPreparsedData(h, ctypes.byref(preparsed)):
                        hid.HidP_GetCaps(preparsed, ctypes.byref(caps))
                        hid.HidD_FreePreparsedData(preparsed)
                        if caps.UsagePage == 0xFF31 and caps.Usage == 0x0079:
                            matched_path = dev_path
                            kernel32.CloseHandle(h)
                            break
                    kernel32.CloseHandle(h)
        index += 1

    setupapi.SetupDiDestroyDeviceInfoList(dev_info)
    return matched_path

path = find_aura_col05()
print(f"Dynamic discovery found Col05 path:\n{path}")
