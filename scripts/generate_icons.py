"""Generate SpaceForge application icons from public/icon.svg.

Builds:
1. Multi-platform icons via Tauri resvg pipeline (PNGs, macOS ICNS, Appx, mobile).
2. Professional 12-frame Windows .ico container supporting all Windows DPI modes:
   [16x16, 20x20, 24x24, 30x30, 32x32, 40x40, 48x48, 64x64, 72x72, 96x96, 128x128, 256x256].
3. High-DPI installer and desktop assets (installer-icon.png, 256x256.png).
"""

import os
import shutil
import struct
import subprocess
from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
OUT = os.path.join(ROOT, "src-tauri", "icons")
SVG = os.path.join(ROOT, "public", "icon.svg")


def main():
    print(f"Generating platform icons from public/icon.svg in {ROOT}...")
    subprocess.run(["npx", "tauri", "icon", "public/icon.svg"], cwd=ROOT, check=True, shell=True)

    icon_png = os.path.join(OUT, "icon.png")
    if not os.path.exists(icon_png):
        raise FileNotFoundError(f"Missing {icon_png}")

    master = Image.open(icon_png).convert("RGBA")

    def make_dib_frame(img):
        w, h = img.size
        img = img.convert("RGBA")
        xor_data = bytearray()
        for y in range(h - 1, -1, -1):
            for x in range(w):
                r, g, b, a = img.getpixel((x, y))
                xor_data.extend([b, g, r, a])
        and_row_bytes = (w + 31) // 32 * 4
        and_data = bytearray()
        for y in range(h - 1, -1, -1):
            row = bytearray(and_row_bytes)
            for x in range(w):
                _, _, _, a = img.getpixel((x, y))
                if a == 0:
                    row[x // 8] |= (1 << (7 - (x % 8)))
            and_data.extend(row)
        bih = struct.pack(
            "<IiiHHIIiiII",
            40,
            w,
            h * 2,
            1,
            32,
            0,
            len(xor_data) + len(and_data),
            0,
            0,
            0,
            0,
        )
        return bih + bytes(xor_data) + bytes(and_data)

    def make_png_frame(img):
        import io
        bio = io.BytesIO()
        img.save(bio, format="PNG")
        return bio.getvalue()

    sizes = [16, 20, 24, 30, 32, 40, 48, 64, 72, 96, 128, 256]
    frames = []
    for s in sizes:
        resized = master.resize((s, s), Image.Resampling.LANCZOS)
        if s == 256:
            raw = make_png_frame(resized)
        else:
            raw = make_dib_frame(resized)
        frames.append((s, raw))

    ico_header = struct.pack("<HHH", 0, 1, len(frames))
    entries = []
    offset = 6 + len(frames) * 16
    for s, raw in frames:
        w_byte = 0 if s >= 256 else s
        h_byte = 0 if s >= 256 else s
        entry = struct.pack("<BBBBHHII", w_byte, h_byte, 0, 0, 1, 32, len(raw), offset)
        entries.append(entry)
        offset += len(raw)

    ico_bytes = ico_header + b"".join(entries) + b"".join(raw for _, raw in frames)
    ico_path = os.path.join(OUT, "icon.ico")
    with open(ico_path, "wb") as f:
        f.write(ico_bytes)

    # Verify binary ICO header
    with open(ico_path, "rb") as f:
        reserved, ico_type, count = struct.unpack("<HHH", f.read(6))
        print(f"Built professional multi-resolution Windows icon bundle ({count} frames):")
        for i in range(count):
            w, h, colors, res, planes, bpp, size, offset = struct.unpack("<BBBBHHII", f.read(16))
            w = 256 if w == 0 else w
            h = 256 if h == 0 else h
            print(f"  - Frame {i:02d}: {w}x{h} ({bpp} bpp)")

    # Ensure desktop and installer PNG assets
    src_256 = os.path.join(OUT, "128x128@2x.png")
    if os.path.exists(src_256):
        shutil.copy(src_256, os.path.join(OUT, "256x256.png"))
        shutil.copy(src_256, os.path.join(OUT, "installer-icon.png"))

    print("Success! High-DPI adaptive multi-resolution icons ready.")


if __name__ == "__main__":
    main()
