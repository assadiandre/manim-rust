"""CE-style animation constructors. Compiled to data at `Scene.play` time."""

from __future__ import annotations

from manim_rust.constants import DEFAULT_MOBJECT_TO_MOBJECT_BUFFER, RIGHT, YELLOW, as_xy, dir_name
from manim_rust.mobject import _node_id


class Animation:
    kind = "create"

    def __init__(self, mobject, run_time=1.0, rate_func="smooth"):
        self.mobject = mobject
        self.run_time = run_time
        self.rate_func = rate_func

    def _spec(self, raw, run_time=None, rate_func=None):
        duration = run_time if run_time is not None else self.run_time
        easing = rate_func if rate_func is not None else self.rate_func
        target = _node_id(self.mobject, raw) if self.mobject is not None else 0
        return (self.kind, target, duration, easing, 0.0, 0.0, "")


class Create(Animation):
    kind = "create"


class Uncreate(Animation):
    kind = "uncreate"


class FadeIn(Animation):
    kind = "fade_in"


class FadeOut(Animation):
    kind = "fade_out"


class Write(Animation):
    kind = "write"


class GrowFromCenter(Animation):
    kind = "grow"


class Indicate(Animation):
    kind = "indicate"


class Wiggle(Animation):
    kind = "wiggle"


class ShrinkToCenter(Animation):
    kind = "shrink"


class SpinInFromNothing(Animation):
    kind = "spin_in"


class DrawBorderThenFill(Animation):
    kind = "draw_border_then_fill"


class ShowPassingFlash(Animation):
    kind = "show_passing_flash"


class Shift(Animation):
    kind = "shift"

    def __init__(self, mobject, delta, **kwargs):
        super().__init__(mobject, **kwargs)
        self.delta = delta

    def _spec(self, raw, run_time=None, rate_func=None):
        kind, target, duration, easing, _, _, extra = super()._spec(raw, run_time, rate_func)
        dx, dy = as_xy(self.delta)
        return (kind, target, duration, easing, dx, dy, extra)


class Scale(Animation):
    kind = "scale"

    def __init__(self, mobject, factor, **kwargs):
        super().__init__(mobject, **kwargs)
        self.factor = factor

    def _spec(self, raw, run_time=None, rate_func=None):
        kind, target, duration, easing, _, _, extra = super()._spec(raw, run_time, rate_func)
        return (kind, target, duration, easing, float(self.factor), 0.0, extra)


class Rotate(Animation):
    kind = "rotate"

    def __init__(self, mobject, angle, **kwargs):
        super().__init__(mobject, **kwargs)
        self.angle = angle

    def _spec(self, raw, run_time=None, rate_func=None):
        kind, target, duration, easing, _, _, extra = super()._spec(raw, run_time, rate_func)
        return (kind, target, duration, easing, float(self.angle), 0.0, extra)


class Recolor(Animation):
    kind = "recolor"

    def __init__(self, mobject, color, **kwargs):
        super().__init__(mobject, **kwargs)
        self.color = color

    def _spec(self, raw, run_time=None, rate_func=None):
        kind, target, duration, easing, a, b, _ = super()._spec(raw, run_time, rate_func)
        return (kind, target, duration, easing, a, b, self.color)


class Circumscribe(Animation):
    kind = "circumscribe"

    def __init__(self, mobject, color=YELLOW, **kwargs):
        super().__init__(mobject, **kwargs)
        self.color = color

    def _spec(self, raw, run_time=None, rate_func=None):
        kind, target, duration, easing, a, b, _ = super()._spec(raw, run_time, rate_func)
        return (kind, target, duration, easing, a, b, self.color)


class GrowFromPoint(Animation):
    kind = "grow_from_point"

    def __init__(self, mobject, point, **kwargs):
        super().__init__(mobject, **kwargs)
        self.point = point

    def _spec(self, raw, run_time=None, rate_func=None):
        kind, target, duration, easing, _, _, extra = super()._spec(raw, run_time, rate_func)
        x, y = as_xy(self.point)
        return (kind, target, duration, easing, x, y, extra)


class GrowFromEdge(Animation):
    kind = "grow_from_edge"

    def __init__(self, mobject, edge, **kwargs):
        super().__init__(mobject, **kwargs)
        self.edge = edge

    def _spec(self, raw, run_time=None, rate_func=None):
        kind, target, duration, easing, _, _, extra = super()._spec(raw, run_time, rate_func)
        x, y = as_xy(self.edge)
        return (kind, target, duration, easing, x, y, extra)


class MoveAlongPath(Animation):
    kind = "move_along_path"

    def __init__(self, mobject, path, **kwargs):
        super().__init__(mobject, **kwargs)
        self.path = path

    def _spec(self, raw, run_time=None, rate_func=None):
        kind, target, duration, easing, _, _, extra = super()._spec(raw, run_time, rate_func)
        path = _node_id(self.path, raw)
        return (kind, target, duration, easing, float(path), 0.0, extra)


class Transform(Animation):
    kind = "morph"


class ReplacementTransform(Transform):
    pass

    def __init__(self, mobject, target_mobject, **kwargs):
        super().__init__(mobject, **kwargs)
        self.target_mobject = target_mobject

    def _spec(self, raw, run_time=None, rate_func=None):
        kind, target, duration, easing, _, _, extra = super()._spec(raw, run_time, rate_func)
        other = _node_id(self.target_mobject, raw)
        return (kind, target, duration, easing, float(other), 0.0, extra)


class TransformMatchingShapes(Animation):
    kind = "transform_matching"

    def __init__(self, mobject, target_mobject, **kwargs):
        super().__init__(mobject, **kwargs)
        self.target_mobject = target_mobject

    def _spec(self, raw, run_time=None, rate_func=None):
        kind, target, duration, easing, _, _, extra = super()._spec(raw, run_time, rate_func)
        other = _node_id(self.target_mobject, raw)
        return (kind, target, duration, easing, float(other), 0.0, extra)


class TransformMatchingTex(TransformMatchingShapes):
    """Same pairing as shapes: Typst glyphs have no TeX-string map yet."""


class FadeTransform(Animation):
    kind = "fade_transform"

    def __init__(self, mobject, target_mobject, **kwargs):
        super().__init__(mobject, **kwargs)
        self.target_mobject = target_mobject

    def _spec(self, raw, run_time=None, rate_func=None):
        kind, target, duration, easing, _, _, extra = super()._spec(raw, run_time, rate_func)
        other = _node_id(self.target_mobject, raw)
        return (kind, target, duration, easing, float(other), 0.0, extra)


class Flash(Animation):
    kind = "flash"

    def __init__(self, point, color=YELLOW, run_time=1.0, rate_func="smooth"):
        super().__init__(None, run_time=run_time, rate_func=rate_func)
        self.point = point
        self.color = color

    def _spec(self, raw, run_time=None, rate_func=None):
        duration = run_time if run_time is not None else self.run_time
        easing = rate_func if rate_func is not None else self.rate_func
        x, y = as_xy(self.point)
        return (self.kind, 0, duration, easing, x, y, self.color)


class Succession:
    def __init__(self, *anims, **kwargs):
        self.anims = anims
        self.run_time = kwargs.get("run_time")
        self.rate_func = kwargs.get("rate_func")


class _BoundAnim(Animation):
    """Result of `mobject.animate.shift(...)` and friends."""

    def __init__(self, mobject, kind, run_time, rate_func, **payload):
        super().__init__(mobject, run_time=run_time, rate_func=rate_func)
        self.kind = kind
        self.payload = payload

    def _spec(self, raw, run_time=None, rate_func=None):
        duration = run_time if run_time is not None else self.run_time
        easing = rate_func if rate_func is not None else self.rate_func
        target = _node_id(self.mobject, raw)
        kind = self.kind
        a = 0.0
        b = 0.0
        extra = ""
        if kind == "shift":
            a, b = as_xy(self.payload["delta"])
        elif kind == "scale":
            a = float(self.payload["factor"])
        elif kind == "rotate":
            a = float(self.payload["angle"])
        elif kind == "recolor":
            extra = self.payload["color"]
        elif kind == "next_to":
            other = _node_id(self.payload["other"], raw)
            dx, dy = raw.next_to_delta(
                target,
                other,
                dir_name(self.payload.get("direction", RIGHT)),
                self.payload.get("buff", DEFAULT_MOBJECT_TO_MOBJECT_BUFFER),
            )
            kind = "shift"
            a, b = dx, dy
        elif kind == "move_to":
            x, y = as_xy(self.payload["point"])
            cx, cy = raw.center_of(target)
            kind = "shift"
            a, b = x - cx, y - cy
        elif kind == "changing_decimal":
            a = float(self.payload["from_value"])
            b = float(self.payload["to_value"])
            extra = str(int(self.payload.get("places", 2)))
            self.mobject.value = b
        return (kind, target, duration, easing, a, b, extra)
