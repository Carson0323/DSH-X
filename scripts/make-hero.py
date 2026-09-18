"""Render docs/hero.png — the banner at the top of the README.

    python scripts/make-hero.py

Style follows the docs landing page: near-black background with the same soft
radial glow, the app icon, and the product name.  The icon comes from
assets/icon.png, so regenerating the icon and re-running this keeps the banner
in sync.
"""

from PIL import Image, ImageDraw, ImageFont
import importlib.util
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ICON = os.path.join(ROOT, "assets", "icon.png")
OUT = os.path.join(ROOT, "docs", "hero.png")

W, H = 1536, 1024
BG = (5, 7, 11)
GLOW = (255, 255, 255)
GLOW_ALPHA = 0.05
ICON_SIZE = 320
TITLE = "DSH-X"
TAGLINE = "选一个版本，启动 dsh web"
TITLE_FONT = r"C:\Windows\Fonts\segoeuib.ttf"
TAGLINE_FONT = r"C:\Windows\Fonts\MiSans-Regular.otf"


def rounded_icon(size):
    """assets/icon.png is the square master; reuse make-icons.py's corner mask."""
    spec = importlib.util.spec_from_file_location("make_icons", os.path.join(ROOT, "scripts", "make-icons.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.fit(size, Image.open(ICON).convert("RGBA"))


def radial_glow(size, center, radius, alpha):
    """Cheap radial gradient: draw at low res and upscale."""
    small = Image.new("L", (size[0] // 8, size[1] // 8), 0)
    px = small.load()
    sw, sh = small.size
    cx, cy = center[0] / 8, center[1] / 8
    rx, ry = radius[0] / 8, radius[1] / 8
    for y in range(sh):
        for x in range(sw):
            d = (((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2) ** 0.5
            if d < 1:
                px[x, y] = int(alpha * 255 * (1 - d) ** 1.5)
    return small.resize(size, Image.Resampling.BILINEAR)


def main():
    base = Image.new("RGB", (W, H), BG)
    glow = radial_glow((W, H), (W * 0.16, H * 0.42), (720, 480), GLOW_ALPHA)
    base.paste(Image.new("RGB", (W, H), GLOW), (0, 0), glow)

    icon = rounded_icon(512).resize((ICON_SIZE, ICON_SIZE), Image.Resampling.LANCZOS)
    x = (W - ICON_SIZE) // 2
    base.paste(icon, (x, 196), icon)

    draw = ImageDraw.Draw(base)
    title_font = ImageFont.truetype(TITLE_FONT, 132)
    tag_font = ImageFont.truetype(TAGLINE_FONT, 44)

    box = draw.textbbox((0, 0), TITLE, font=title_font)
    title_y = 196 + ICON_SIZE + 74 - box[1]
    draw.text(((W - (box[2] - box[0])) / 2 - box[0], title_y), TITLE, font=title_font, fill=(255, 255, 255))

    tag_box = draw.textbbox((0, 0), TAGLINE, font=tag_font)
    tag_y = title_y + box[3] + 42
    draw.text(
        ((W - (tag_box[2] - tag_box[0])) / 2 - tag_box[0], tag_y),
        TAGLINE,
        font=tag_font,
        fill=(255, 255, 255, 140),
    )

    base.save(OUT)
    print("hero", OUT, base.size, os.path.getsize(OUT))


if __name__ == "__main__":
    main()
