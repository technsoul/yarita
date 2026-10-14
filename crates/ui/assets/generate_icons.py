"""Rasterizes the logo (white mark on black) into the app icon files.

Mirrors logo.svg / icon.svg on their 200x200 grid. Run from anywhere:
    python crates/ui/assets/generate_icons.py
"""

import math
from pathlib import Path

from PIL import Image, ImageDraw

HERE = Path(__file__).parent
SS = 4  # supersampling factor
CENTER, RADIUS = (100.0, 100.0), 68.0
RING = [((64, 42.3), (139, 44.3)), ((165.7, 82.4), (134, 158.9)), ((66, 158.9), (36.1, 76.7))]
HANDS = [[(47.6, 63.3), (100, 100), (163.9, 55.2)], [(100, 100), (100, 170)]]


def angle(p):
    return math.degrees(math.atan2(p[1] - CENTER[1], p[0] - CENTER[0])) % 360


def render(size, corner_radius=44):
    big = size * SS
    s = big / 200
    img = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle((0, 0, big - 1, big - 1), radius=corner_radius * s, fill="black")

    def dot(p, r):
        d.ellipse(((p[0] - r) * s, (p[1] - r) * s, (p[0] + r) * s, (p[1] + r) * s), fill="white")

    box = [(CENTER[0] - RADIUS - 6) * s, (CENTER[1] - RADIUS - 6) * s,
           (CENTER[0] + RADIUS + 6) * s, (CENTER[1] + RADIUS + 6) * s]
    for a, b in RING:
        start, end = angle(a), angle(b)
        if end < start:
            end += 360
        # Sweep flag 1 is clockwise on screen, which is PIL's direction.
        d.arc(box, start, end, fill="white", width=round(12 * s))
        dot(a, 6)
        dot(b, 6)
    for pts in HANDS:
        d.line([(x * s, y * s) for x, y in pts], fill="white", width=round(14 * s), joint="curve")
        for p in pts:
            dot(p, 7)
    dot(CENTER, 8.5)
    return img.resize((size, size), Image.LANCZOS)


icon = render(1024)
icon.save(HERE / "icon.png")
render(256).save(HERE / "icon-256.png")
icon.save(HERE / "icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
icon.save(HERE / "icon.icns")
