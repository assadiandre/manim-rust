"""Directions, buffers, and ManimCE color names.

Color values are palette keys understood by the Rust `palette::named`
lookup. `YELLOW` is CE's `YELLOW_C` (`yellow_c`); the hex `#FFFF00`
literal stays available as `PURE_YELLOW` so older goldens do not shift.
"""

from __future__ import annotations

import math

try:
    from manim_rust._native import (
        DEFAULT_MOBJECT_TO_EDGE_BUFFER,
        DEFAULT_MOBJECT_TO_MOBJECT_BUFFER,
        DL,
        DOWN,
        DR,
        LEFT,
        ORIGIN,
        RIGHT,
        UL,
        UP,
        UR,
    )
except ImportError as exc:  # pragma: no cover
    raise ImportError(
        "manim_rust native extension is not installed. "
        "Run: maturin develop -m crates/manim-py/Cargo.toml"
    ) from exc

PI = math.pi
TAU = 2.0 * math.pi
DEGREES = math.pi / 180.0

WHITE = "white"
BLACK = "black"
BLUE = "blue"
BLUE_A = "blue_a"
BLUE_B = "blue_b"
BLUE_C = "blue_c"
BLUE_D = "blue_d"
BLUE_E = "blue_e"
DARK_BLUE = "dark_blue"
TEAL = "teal"
TEAL_A = "teal_a"
TEAL_B = "teal_b"
TEAL_C = "teal_c"
TEAL_D = "teal_d"
TEAL_E = "teal_e"
GREEN = "green"
GREEN_A = "green_a"
GREEN_B = "green_b"
GREEN_C = "green_c"
GREEN_D = "green_d"
GREEN_E = "green_e"
YELLOW_A = "yellow_a"
YELLOW_B = "yellow_b"
YELLOW_C = "yellow_c"
YELLOW_D = "yellow_d"
YELLOW_E = "yellow_e"
YELLOW = YELLOW_C
PURE_YELLOW = "yellow"
GOLD = "gold"
GOLD_A = "gold_a"
GOLD_B = "gold_b"
GOLD_C = "gold_c"
GOLD_D = "gold_d"
GOLD_E = "gold_e"
RED = "red"
RED_A = "red_a"
RED_B = "red_b"
RED_C = "red_c"
RED_D = "red_d"
RED_E = "red_e"
MAROON = "maroon"
MAROON_A = "maroon_a"
MAROON_B = "maroon_b"
MAROON_C = "maroon_c"
MAROON_D = "maroon_d"
MAROON_E = "maroon_e"
PURPLE = "purple"
PURPLE_A = "purple_a"
PURPLE_B = "purple_b"
PURPLE_C = "purple_c"
PURPLE_D = "purple_d"
PURPLE_E = "purple_e"
PINK = "pink"
LIGHT_PINK = "light_pink"
ORANGE = "orange"
GRAY = "gray"
GREY = "grey"
GRAY_A = "gray_a"
GRAY_B = "gray_b"
GRAY_C = "gray_c"
GRAY_D = "gray_d"
GRAY_E = "gray_e"

_DIR_NAMES = {
    (0.0, 0.0): "origin",
    (0.0, 1.0): "up",
    (0.0, -1.0): "down",
    (-1.0, 0.0): "left",
    (1.0, 0.0): "right",
    (-1.0, 1.0): "ul",
    (1.0, 1.0): "ur",
    (-1.0, -1.0): "dl",
    (1.0, -1.0): "dr",
}


def _sgn(v: float) -> float:
    if v > 0.5:
        return 1.0
    if v < -0.5:
        return -1.0
    return 0.0


def as_xy(value) -> tuple[float, float]:
    if isinstance(value, (int, float)):
        return (float(value), 0.0)
    return (float(value[0]), float(value[1]))


def dir_name(direction) -> str:
    if isinstance(direction, str):
        return direction
    x, y = as_xy(direction)
    return _DIR_NAMES.get((_sgn(x), _sgn(y)), "right")
