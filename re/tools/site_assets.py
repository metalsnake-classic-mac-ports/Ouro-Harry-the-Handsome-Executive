"""Make the images of the website (docs/site/) from the data pack and from
screens the game exports.

    for l in 1 2 7 9 11; do ouro-harry --screenshot shots/l$l.bmp $l 40; done
    re/work/venv/bin/python3 -I re/tools/site_assets.py data/ouro-harry-data.zip shots docs/site re/work/res

Writes: logo.png (the logo from the game screen's panel, on its granite),
granite.png (a tile of the panel's stone), menu.png (the main menu
picture), things.png (the instructions' "things at ScumCo"), credits.png
(the credits list), title.png (the title picture, Harry Titles' PICT 128,
from the dumped resources - the port does not show it, so the pack does not
have it - without its "Loading" bar), screen-*.png (the port's level
screens, 640 x 480) and harry.png (Harry on his chair, the icon). Needs
Pillow.
"""
import io
import os
import sys
import zipfile

from PIL import Image

_tools = os.environ.get("OURO_TOOLS") or os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "ouro-tools")
sys.path.insert(0, os.path.join(_tools, "re"))   # the shared library (ouro-tools/re)
from pict import decode  # noqa: E402

# the title picture down to its "Loading" bar (rows 416 on)
TITLE_ROWS = 416

SCREENS = {1: "screen-level1.png", 2: "screen-level2.png", 7: "screen-level7.png", 9: "screen-level9.png",
           11: "screen-level11.png"}


def main():
    pack, shots, out, res = sys.argv[1:5]
    os.makedirs(out, exist_ok=True)
    z = zipfile.ZipFile(pack)

    def image(name):
        return Image.open(io.BytesIO(z.read(name))).convert("RGB")

    game = image("pictures/129.png")                  # the game screen: panel on the right
    game.crop((466, 14, 626, 88)).save(f"{out}/logo.png", optimize=True)
    game.crop((480, 170, 608, 298)).save(f"{out}/granite.png", optimize=True)
    image("pictures/130.png").save(f"{out}/menu.png", optimize=True)
    image("pictures/132.png").save(f"{out}/things.png", optimize=True)
    image("pictures/4803.png").save(f"{out}/credits.png", optimize=True)
    face = Image.open(io.BytesIO(z.read("faces/1110.png"))).convert("RGBA")
    icon = Image.new("RGBA", (48, 48))
    icon.alpha_composite(face, ((48 - face.width) // 2, (48 - face.height) // 2))
    icon.save(f"{out}/harry.png", optimize=True)
    w, h, rows, pal = decode(open(f"{res}/Harry_Titles/PICT_128.bin", "rb").read())
    title = Image.frombytes("RGB", (w, h), b"".join(b"".join(bytes(pal[i]) for i in r) for r in rows))
    title.crop((0, 0, w, TITLE_ROWS)).save(f"{out}/title.png", optimize=True)
    for level, name in SCREENS.items():
        Image.open(f"{shots}/l{level}.bmp").convert("RGB").save(f"{out}/{name}", optimize=True)
    print("written to", out)


if __name__ == "__main__":
    main()
