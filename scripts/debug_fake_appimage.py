#!/usr/bin/env python3
"""Depura la AppImage simulada y el flujo de extraccion de NaranjaHub."""
import base64, os, stat, subprocess, sys

PNG = base64.b64decode(
    'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg=='
)
B64 = base64.b64encode(PNG).decode()

WORK = "/tmp/simtest"
ROOT = os.path.join(WORK, "squashfs-root")

SCRIPT = f"""#!/bin/sh
if [ "$1" = "--appimage-extract" ]; then
    mkdir -p squashfs-root/usr/share/applications \\
      squashfs-root/usr/share/icons/hicolor/256x256/apps
    printf '%s' "{B64}" | base64 -d > \\
      squashfs-root/usr/share/icons/hicolor/256x256/apps/demo-sim.png
    ln -sf usr/share/icons/hicolor/256x256/apps/demo-sim.png \\
      squashfs-root/.DirIcon
    cat > squashfs-root/demo-sim.desktop <<'NHEOF'
[Desktop Entry]
Type=Application
Name=Demo Simulada
Exec=demo-sim
Icon=demo-sim
X-AppImage-Version=3.1
NHEOF
    cp squashfs-root/demo-sim.desktop \\
      squashfs-root/usr/share/applications/demo-sim.desktop
    exit 0
fi
echo "ejecutando demo-sim"
exit 0
"""

os.makedirs(WORK, exist_ok=True)
appimage = os.path.join(WORK, "Demo.AppImage")
with open(appimage, "w") as f:
    f.write(SCRIPT)
os.chmod(appimage, 0o755)

r = subprocess.run([appimage, "--appimage-extract"], cwd=WORK,
                   capture_output=True, text=True, timeout=30)
print("extract exit:", r.returncode, "| stdout:", r.stdout[:80], "| stderr:", r.stderr[:80])
for path in [
    f"{ROOT}/.DirIcon",
    f"{ROOT}/demo-sim.png" if False else f"{ROOT}/usr/share/icons/hicolor/256x256/apps/demo-sim.png",
    f"{ROOT}/demo-sim.desktop",
]:
    ok = os.path.exists(path)
    print(f"  {path}: {'OK' if ok else 'FALTA'}")
print("  .DirIcon ->", os.readlink(f"{ROOT}/.DirIcon"))
with open(f"{ROOT}/usr/share/icons/hicolor/256x256/apps/demo-sim.png", "rb") as f:
    head = f.read(8)
print("  png magic:", head[:8] == b"\x89PNG\r\n\x1a\n")
print("SIMULACION OK" if r.returncode == 0 else "SIMULACION FALLO")
sys.exit(0 if r.returncode == 0 else 1)
