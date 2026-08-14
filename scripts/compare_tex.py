"""LaTeX formula parity: manim_rust (mitex->typst) vs ManimCE (real LaTeX).

Renders each formula as a still with both engines, stacks
[ours | ManimCE | amplified diff], computes numeric diff metrics, and writes
an HTML contact sheet for human review. Same review-aid caveat as
compare.py: different typesetting engines (New Computer Modern via typst vs
Computer Modern via latex) guarantee nonzero diffs even when both are right.

Usage (from scripts/):
    ../.venv-ref/bin/python compare_tex.py --out ../media/visual_check/tex_parity
"""

import argparse
import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

sys.path.insert(0, str(Path(__file__).parent))
from tex_reference import FORMULAS

THUMB_W = 480
SCRIPTS = Path(__file__).parent


def render_theirs() -> None:
    scenes = [f"TexRef{i}" for i in range(len(FORMULAS))]
    subprocess.run(
        [sys.executable, "-m", "manim", "-qh", "-s", "tex_reference.py", *scenes],
        cwd=SCRIPTS,
        check=True,
    )


def render_ours(out: Path) -> None:
    from manim_rust import Scene

    for i, formula in enumerate(FORMULAS):
        scene = Scene(1920, 1080)
        scene.add_tex(formula)
        scene.save_png(str(out / f"ours_{i}.png"), time=0.0)


def thumb(img: Image.Image) -> Image.Image:
    h = round(img.height * THUMB_W / img.width)
    return img.resize((THUMB_W, h), Image.LANCZOS)


def label(img: Image.Image, text: str) -> Image.Image:
    bar = Image.new("RGB", (img.width, 22), (24, 24, 24))
    ImageDraw.Draw(bar).text((6, 4), text, fill=(230, 230, 230))
    out = Image.new("RGB", (img.width, img.height + 22))
    out.paste(bar, (0, 0))
    out.paste(img, (0, 22))
    return out


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    render_theirs()
    render_ours(args.out)

    rows_html = []
    for i, formula in enumerate(FORMULAS):
        a = Image.open(args.out / f"ours_{i}.png").convert("RGB")
        b = Image.open(SCRIPTS / f"media/images/tex_reference/TexRef{i}.png").convert("RGB")
        if a.size != b.size:
            b = b.resize(a.size, Image.LANCZOS)

        ad = np.abs(np.asarray(a, dtype=np.int16) - np.asarray(b, dtype=np.int16))
        frac = float((ad > 8).mean())
        mean = float(ad.mean())
        diff_img = Image.fromarray(np.clip(ad * 4, 0, 255).astype(np.uint8))

        row = Image.new("RGB", (THUMB_W * 3 + 16, thumb(a).height + 22), (12, 12, 12))
        row.paste(label(thumb(a), "manim_rust (mitex)"), (0, 0))
        row.paste(label(thumb(b), "ManimCE (latex)"), (THUMB_W + 8, 0))
        row.paste(
            label(thumb(diff_img), f"diff x4   {frac:.1%} px differ, mean {mean:.2f}"),
            (2 * (THUMB_W + 8), 0),
        )
        name = f"row_{i}.png"
        row.save(args.out / name)
        rows_html.append(
            f"<h2><code>{formula}</code> — {frac:.1%} px differ, mean |Δ| {mean:.2f}</h2>"
            f'<img src="{name}" style="width:100%;max-width:1500px">'
        )
        print(f"[{i}] {formula[:40]:42} differ>8: {frac:6.2%}   mean|Δ|: {mean:5.2f}")

    html = (
        "<!doctype html><meta charset=utf-8><title>LaTeX parity</title>"
        "<style>body{background:#111;color:#eee;font:14px system-ui;margin:2em}"
        "h2{font-size:1em;margin:1.5em 0 .4em}</style>"
        "<h1>manim_rust mitex vs ManimCE latex</h1>" + "".join(rows_html)
    )
    (args.out / "index.html").write_text(html)
    print(f"wrote {args.out / 'index.html'}")


if __name__ == "__main__":
    main()
