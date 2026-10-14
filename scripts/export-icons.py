"""Export the selected Mystline SVG using PyGObject/Rsvg, Pycairo and Pillow."""

from io import BytesIO
from pathlib import Path
import re

import cairo
import gi
from PIL import Image

gi.require_version("Rsvg", "2.0")
from gi.repository import Rsvg


ROOT = Path(__file__).resolve().parents[1]
MARK = ROOT / "frontend/public/mystline-mark.svg"
ICONS = ROOT / "src-tauri/icons"
DESIGNS = ROOT / "docs/designs/logos"


def viewport(width: int, height: int) -> Rsvg.Rectangle:
    rectangle = Rsvg.Rectangle()
    rectangle.x = rectangle.y = 0
    rectangle.width = width
    rectangle.height = height
    return rectangle


def render(svg: bytes, width: int, height: int) -> Image.Image:
    surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, width, height)
    handle = Rsvg.Handle.new_from_data(svg)
    handle.render_document(cairo.Context(surface), viewport(width, height))
    data = BytesIO()
    surface.write_to_png(data)
    data.seek(0)
    return Image.open(data).convert("RGBA")


def main() -> None:
    svg = MARK.read_bytes()
    # Supersampling keeps rounded caps smooth at tray sizes.
    image = render(svg, 1024, 1024)
    image.resize((512, 512), Image.Resampling.LANCZOS).save(ICONS / "icon.png")
    image.resize((256, 256), Image.Resampling.LANCZOS).save(
        ICONS / "icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    )
    tray = image.resize((32, 32), Image.Resampling.LANCZOS)
    tray.save(ICONS / "tray-32.png")
    (ICONS / "tray-32.rgba").write_bytes(tray.tobytes())
    (DESIGNS / "mystline-mark.svg").write_bytes(svg)

    # Outline the wordmark so the exported logo needs no installed font.
    path = re.search(r'<path\b[^>]*/>', svg.decode()).group(0)
    logo = (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 192">'
        '<g transform="translate(24 24) scale(2.25)" fill="none">'
        + path
        + '</g><text x="198" y="122" fill="#293247" font-family="Noto Sans" '
        'font-size="78" font-weight="700">Mystline</text></svg>'
    )
    with cairo.SVGSurface(str(DESIGNS / "mystline-logo.svg"), 640, 192) as surface:
        Rsvg.Handle.new_from_data(logo.encode()).render_document(
            cairo.Context(surface), viewport(640, 192)
        )
    render((DESIGNS / "mystline-logo.svg").read_bytes(), 1280, 384).save(DESIGNS / "mystline-logo.png")


if __name__ == "__main__":
    main()
