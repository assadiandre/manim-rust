"""Side-by-side comparison: manim_rust vs ManimCE reference.

Extracts frames at semantically matching timestamps from both renders,
stacks [ours | ManimCE | amplified diff] per timestamp, computes numeric
diff metrics, and writes an HTML contact sheet for human review.

The metrics are a review aid, NOT a CI assertion: AA, font, and bezier
subdivision differences guarantee nonzero diffs even when both are correct.

Usage:
    ../.venv-ref/bin/python compare.py \
        --ours ../media/demo.mp4 \
        --theirs media/videos/manimce_reference/1080p60/RustDemoReference.mp4 \
        --out ../media/visual_check/sidebyside
"""

import argparse
import subprocess
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

TIMES = [0.5, 1.0, 1.6, 2.2, 2.6, 3.0, 3.45]
THUMB_W = 480


def grab(video: Path, t: float, out: Path) -> None:
    subprocess.run(
        ["ffmpeg", "-v", "error", "-ss", f"{t}", "-i", str(video),
         "-frames:v", "1", str(out), "-y"],
        check=True,
    )


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
    ap.add_argument("--ours", type=Path, required=True)
    ap.add_argument("--theirs", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()

    args.out.mkdir(parents=True, exist_ok=True)
    rows_html = []

    for t in TIMES:
        a_path, b_path = args.out / f"a_{t}.png", args.out / f"b_{t}.png"
        grab(args.ours, t, a_path)
        grab(args.theirs, t, b_path)
        a = Image.open(a_path).convert("RGB")
        b = Image.open(b_path).convert("RGB")
        if a.size != b.size:
            b = b.resize(a.size, Image.LANCZOS)

        d = np.asarray(a, dtype=np.int16) - np.asarray(b, dtype=np.int16)
        ad = np.abs(d)
        frac = float((ad > 8).mean())
        mean = float(ad.mean())
        diff_img = Image.fromarray((np.clip(ad * 4, 0, 255)).astype(np.uint8))

        row = Image.new("RGB", (THUMB_W * 3 + 16, thumb(a).height + 22), (12, 12, 12))
        row.paste(label(thumb(a), f"manim_rust  t={t}"), (0, 0))
        row.paste(label(thumb(b), "ManimCE (fork)"), (THUMB_W + 8, 0))
        row.paste(label(thumb(diff_img), f"diff x4   {frac:.1%} px differ, mean {mean:.2f}"),
                  (2 * (THUMB_W + 8), 0))
        name = f"row_{t}.png"
        row.save(args.out / name)
        rows_html.append(f"<h2>t = {t}s — {frac:.1%} px differ, mean |Δ| {mean:.2f}</h2>"
                         f'<img src="{name}" style="width:100%;max-width:1500px">')
        print(f"t={t:5.2f}  differ>{8}: {frac:6.2%}   mean|Δ|: {mean:5.2f}")

    html = ("<!doctype html><meta charset=utf-8><title>manim_rust vs ManimCE</title>"
            "<style>body{background:#111;color:#eee;font:14px system-ui;margin:2em}"
            "h2{font-size:1em;margin:1.5em 0 .4em}</style>"
            "<h1>manim_rust vs ManimCE reference</h1>" + "".join(rows_html))
    (args.out / "index.html").write_text(html)
    print(f"wrote {args.out / 'index.html'}")


if __name__ == "__main__":
    main()
