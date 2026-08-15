"""Side-by-side manim_rust goldens vs ManimCE frames for M25–M29 probes.

Writes media/visual_check/sidebyside_m25/row_*.png and index.html.
Metrics are a review aid (AA / adaptive sampling / stroke scaling differ).

Usage (repo root or scripts/):
    python3 scripts/compare_side_by_side.py
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
SCRIPTS = ROOT / "scripts"
GOLDEN = ROOT / "crates/manim-render/tests/golden"
OUT = ROOT / "media/visual_check/sidebyside_m25"
CE_DIR = OUT / "ce"
THUMB_W = 480

# (label, ours_png, ce_scene, ce_kind, ce_time_or_none)
# ce_kind: "still" uses manim -s; "video" extracts a timestamp from the mp4.
CASES = [
    ("implicit", "implicit.png", "ImplicitRef", "still", None),
    ("wave t=0", "wave_0.0.png", "WaveRef", "video", 0.0),
    ("wave t=0.5", "wave_0.5.png", "WaveRef", "video", 0.5),
    ("wave t=1", "wave_1.0.png", "WaveRef", "video", 1.0),
    ("stretch t=0", "stretch_0.0.png", "StretchRef", "video", 0.0),
    ("stretch t=0.6", "stretch_0.6.png", "StretchRef", "video", 0.6),
    ("stretch t=1.2", "stretch_1.2.png", "StretchRef", "video", 1.2),
    ("raster", "raster.png", "RasterRef", "still", None),
    ("arcp", "arcp.png", "ArcpRef", "still", None),
    ("restore t=0", "restore_0.0.png", "RestoreRef", "video", 0.0),
    ("restore t=0.5", "restore_0.5.png", "RestoreRef", "video", 0.5),
    ("restore t=1", "restore_1.0.png", "RestoreRef", "video", 1.0),
    ("become", "become.png", "BecomeRef", "still", None),
    ("boolean", "boolean.png", "BooleanRef", "still", None),
    ("svg", "svg.png", "SvgRef", "still", None),
    ("tfc t=0", "tfc_0.0.png", "TfcRef", "video", 0.0),
    ("tfc t=0.5", "tfc_0.5.png", "TfcRef", "video", 0.5),
    ("tfc t=1", "tfc_1.0.png", "TfcRef", "video", 1.0),
    ("broadcast t=0", "broadcast_0.0.png", "BroadcastRef", "video", 0.0),
    ("broadcast t=0.6", "broadcast_0.6.png", "BroadcastRef", "video", 0.6),
    ("broadcast t=1.5", "broadcast_1.5.png", "BroadcastRef", "video", 1.5),
    ("fadeto t=0", "fadeto_0.0.png", "FadeToRef", "video", 0.0),
    ("fadeto t=0.5", "fadeto_0.5.png", "FadeToRef", "video", 0.5),
    ("fadeto t=1", "fadeto_1.0.png", "FadeToRef", "video", 1.0),
]


def write_assets() -> None:
    assets = SCRIPTS / "ce_assets"
    assets.mkdir(exist_ok=True)
    check = Image.new("RGBA", (8, 8))
    px = check.load()
    for y in range(8):
        for x in range(8):
            px[x, y] = (255, 255, 0, 255) if (x + y) % 2 == 0 else (88, 196, 221, 255)
    check.save(assets / "checker.png")
    grad = Image.new("RGBA", (64, 16))
    gpx = grad.load()
    for y in range(16):
        for x in range(64):
            t = x / 63.0
            gpx[x, y] = (
                int(252 * (1 - t) + 88 * t),
                int(98 * (1 - t) + 196 * t),
                int(85 * (1 - t) + 221 * t),
                255,
            )
    grad.save(assets / "gradient.png")


def render_ce() -> None:
    CE_DIR.mkdir(parents=True, exist_ok=True)
    stills = sorted({c[2] for c in CASES if c[3] == "still"})
    videos = sorted({c[2] for c in CASES if c[3] == "video"})
    py = sys.executable
    if stills:
        subprocess.run(
            [py, "-m", "manim", "-s", "-r", "480,270", "--disable_caching",
             "--media_dir", str(CE_DIR), "ce_parity_refs.py", *stills],
            cwd=SCRIPTS,
            check=True,
        )
    for scene in videos:
        subprocess.run(
            [py, "-m", "manim", "-r", "480,270", "--fps", "10", "--disable_caching",
             "--media_dir", str(CE_DIR), "ce_parity_refs.py", scene],
            cwd=SCRIPTS,
            check=True,
        )


def find_ce_still(scene: str) -> Path:
    hits = [p for p in CE_DIR.rglob("*.png") if scene in p.stem]
    if not hits:
        raise FileNotFoundError(f"CE still for {scene} under {CE_DIR}")
    return hits[0]


def find_ce_video(scene: str) -> Path:
    hits = [p for p in CE_DIR.rglob("*.mp4") if p.stem == scene]
    if not hits:
        raise FileNotFoundError(f"CE video for {scene} under {CE_DIR}")
    return hits[0]


def grab_video(video: Path, t: float, dest: Path) -> None:
    # Seeking exactly to duration often yields no frame; nudge backward.
    for ss in (t, max(0.0, t - 0.05), max(0.0, t - 0.12)):
        dest.unlink(missing_ok=True)
        subprocess.run(
            ["ffmpeg", "-v", "error", "-ss", f"{ss}", "-i", str(video),
             "-frames:v", "1", str(dest), "-y"],
            check=False,
        )
        if dest.exists() and dest.stat().st_size > 0:
            return
    raise RuntimeError(f"ffmpeg produced no frame at t={t} from {video}")


def thumb(img: Image.Image) -> Image.Image:
    h = max(1, round(img.height * THUMB_W / img.width))
    return img.resize((THUMB_W, h), Image.LANCZOS)


def label(img: Image.Image, text: str) -> Image.Image:
    bar = Image.new("RGB", (img.width, 22), (24, 24, 24))
    ImageDraw.Draw(bar).text((6, 4), text, fill=(230, 230, 230))
    out = Image.new("RGB", (img.width, img.height + 22))
    out.paste(bar, (0, 0))
    out.paste(img, (0, 22))
    return out


def compare_pair(ours: Path, theirs: Path, title: str) -> tuple[Image.Image, float, float]:
    a = Image.open(ours).convert("RGB")
    b = Image.open(theirs).convert("RGB")
    if a.size != b.size:
        b = b.resize(a.size, Image.LANCZOS)
    d = np.asarray(a, dtype=np.int16) - np.asarray(b, dtype=np.int16)
    ad = np.abs(d)
    frac = float((ad > 8).mean())
    mean = float(ad.mean())
    diff_img = Image.fromarray(np.clip(ad * 4, 0, 255).astype(np.uint8))
    row = Image.new("RGB", (THUMB_W * 3 + 16, thumb(a).height + 22), (12, 12, 12))
    row.paste(label(thumb(a), f"manim_rust  {title}"), (0, 0))
    row.paste(label(thumb(b), "ManimCE"), (THUMB_W + 8, 0))
    row.paste(
        label(thumb(diff_img), f"diff x4   {frac:.1%} px, mean {mean:.2f}"),
        (2 * (THUMB_W + 8), 0),
    )
    return row, frac, mean


def main() -> None:
    write_assets()
    OUT.mkdir(parents=True, exist_ok=True)
    if "--skip-ce" not in sys.argv:
        render_ce()

    rows_html = []
    for title, ours_name, scene, kind, t in CASES:
        ours = GOLDEN / ours_name
        if not ours.exists():
            print(f"skip {title}: missing {ours}")
            continue
        if kind == "still":
            theirs = find_ce_still(scene)
        else:
            theirs = OUT / f"ce_{scene}_{t:.1f}.png"
            grab_video(find_ce_video(scene), float(t), theirs)
        row, frac, mean = compare_pair(ours, theirs, title)
        name = f"row_{ours_name}"
        row.save(OUT / name)
        rows_html.append(
            f"<h2>{title} — {frac:.1%} px differ, mean |Δ| {mean:.2f}</h2>"
            f'<img src="{name}" style="width:100%;max-width:1500px">'
        )
        print(f"{title:18}  differ>{8}: {frac:6.2%}   mean|Δ|: {mean:5.2f}")

    html = (
        "<!doctype html><meta charset=utf-8><title>M25–M29 vs ManimCE</title>"
        "<style>body{background:#111;color:#eee;font:14px system-ui;margin:2em}"
        "h2{font-size:1em;margin:1.5em 0 .4em}</style>"
        "<h1>manim_rust vs ManimCE (M25–M29 probes)</h1>" + "".join(rows_html)
    )
    (OUT / "index.html").write_text(html)
    print(f"wrote {OUT / 'index.html'}")


if __name__ == "__main__":
    main()
