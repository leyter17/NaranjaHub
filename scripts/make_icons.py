#!/usr/bin/env python3
# NaranjaHub - generador de iconos (naranja/negro)
# Copyright (C) 2026 Leyter Vazquez. V - GNU GPL v3
import os
from PIL import Image, ImageDraw

# scripts/ y naranjahub/ son hermanos dentro de my-project/
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ASSETS = os.path.join(ROOT, "naranjahub", "assets")
MASTER = os.path.join(ASSETS, "logo-master.png")


def crop_content(img: Image.Image) -> Image.Image:
    """Recorta los margenes claros dejando el cuadrado oscuro centrado."""
    rgb = img.convert("RGB")
    w, h = rgb.size
    px = rgb.load()
    minx, miny, maxx, maxy = w, h, 0, 0
    step = 2
    for y in range(0, h, step):
        for x in range(0, w, step):
            r, g, b = px[x, y]
            if r + g + b < 560:  # pixel oscuro (el cuadrado negro)
                minx = min(minx, x)
                miny = min(miny, y)
                maxx = max(maxx, x)
                maxy = max(maxy, y)
    if maxx <= minx or maxy <= miny:
        return img
    # margen de seguridad de 4 px para no cortar el borde redondeado
    pad = 4
    return img.crop((max(0, minx - pad), max(0, miny - pad),
                     min(w, maxx + pad), min(h, maxy + pad)))


def rounded(img: Image.Image, radius_pct: float = 0.22) -> Image.Image:
    """Aplica esquinas redondeadas con transparencia."""
    img = img.convert("RGBA")
    s = min(img.size)
    img = img.resize((s, s), Image.LANCZOS)
    radius = int(s * radius_pct)
    mask = Image.new("L", (s, s), 0)
    d = ImageDraw.Draw(mask)
    d.rounded_rectangle((0, 0, s - 1, s - 1), radius=radius, fill=255)
    img.putalpha(mask)
    return img


def draw_generic_icon(size: int = 128) -> Image.Image:
    """Icono generico para AppImages sin icono extraible: caja naranja sobre negro."""
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    s = size
    rad = int(s * 0.22)
    # fondo negro redondeado
    d.rounded_rectangle((0, 0, s - 1, s - 1), radius=rad, fill=(20, 20, 24, 255))
    # caja naranja
    m = int(s * 0.22)
    bw = s - 2 * m
    box_h = int(bw * 0.68)
    top = int(s * 0.34)
    d.rounded_rectangle((m, top, s - m - 1, top + box_h - 1),
                        radius=int(s * 0.06), fill=(255, 122, 26, 255))
    # tapa de la caja
    lid_h = int(s * 0.12)
    d.rounded_rectangle((m, top - lid_h // 2, s - m - 1, top + lid_h // 2),
                        radius=int(s * 0.05), fill=(255, 146, 61, 255))
    # flecha blanca de lanzamiento
    ax = m + int(bw * 0.30)
    ay = top + int(box_h * 0.30)
    a = int(s * 0.11)
    d.line((ax, ay + a, ax + a, ay), fill=(255, 255, 255, 255), width=max(3, s // 24))
    d.polygon([(ax + a, ay), (ax + a, ay + int(a * 0.45)),
               (ax + a - int(a * 0.45), ay)], fill=(255, 255, 255, 255))
    return img


def main() -> None:
    os.makedirs(ASSETS, exist_ok=True)
    master = Image.open(MASTER)
    logo = rounded(crop_content(master))
    logo.save(os.path.join(ASSETS, "logo.png"))  # 512 aprox, para la interfaz
    for size in (512, 256, 128, 64, 48, 32, 16):
        logo.resize((size, size), Image.LANCZOS).save(
            os.path.join(ASSETS, f"icon-{size}.png"))
    draw_generic_icon(128).save(os.path.join(ASSETS, "generic-app.png"))
    draw_generic_icon(48).save(os.path.join(ASSETS, "generic-app-48.png"))
    print("Iconos generados en", ASSETS)
    for f in sorted(os.listdir(ASSETS)):
        print(" ", f, os.path.getsize(os.path.join(ASSETS, f)), "bytes")


if __name__ == "__main__":
    main()
