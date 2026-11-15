import sys
print("Python:", sys.executable)
for pkg in ["pystray", "PIL", "keyboard"]:
    try:
        __import__(pkg)
        print(f"{pkg}: available")
    except ImportError:
        print(f"{pkg}: NOT available")
