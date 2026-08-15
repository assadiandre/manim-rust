"""Deferred CE-style mobjects. Geometry is created on first add/play."""

from __future__ import annotations

from manim_rust.constants import (
    DOWN,
    RED,
    RIGHT,
    WHITE,
    as_xy,
    dir_name,
)
from manim_rust.mobject import Mobject, VMobject, _node_id


class Polygon(VMobject):
    def __init__(self, *vertices, **kwargs):
        super().__init__(**kwargs)
        self.vertices = vertices

    def _add(self, raw):
        fill, stroke, width = self._style()
        points = [as_xy(v) for v in self.vertices]
        return raw.add_polygon(points, fill=fill, stroke=stroke, stroke_width=width)


class Vector(VMobject):
    def __init__(self, direction=RIGHT, color=WHITE, stroke_width=6.0, **kwargs):
        super().__init__(color=color, stroke_width=stroke_width, **kwargs)
        self.direction = direction

    def _add(self, raw):
        _, stroke, width = self._style()
        x, y = as_xy(self.direction)
        return raw.add_vector(x, y, stroke=stroke, stroke_width=width)


class DoubleArrow(VMobject):
    def __init__(self, start, end, color=WHITE, stroke_width=6.0, buff=0.25, **kwargs):
        super().__init__(color=color, stroke_width=stroke_width, **kwargs)
        self.start = start
        self.end = end
        self.buff = buff

    def _add(self, raw):
        _, stroke, width = self._style()
        x1, y1 = as_xy(self.start)
        x2, y2 = as_xy(self.end)
        return raw.add_double_arrow(
            x1, y1, x2, y2, buff=self.buff, stroke=stroke, stroke_width=width
        )


class DashedLine(VMobject):
    def __init__(self, start, end, color=WHITE, stroke_width=4.0, dash=0.15, gap=0.1, **kwargs):
        super().__init__(color=color, stroke_width=stroke_width, **kwargs)
        self.start = start
        self.end = end
        self.dash = dash
        self.gap = gap

    def _add(self, raw):
        _, stroke, width = self._style()
        x1, y1 = as_xy(self.start)
        x2, y2 = as_xy(self.end)
        return raw.add_dashed_line(
            x1,
            y1,
            x2,
            y2,
            dash=self.dash,
            gap=self.gap,
            stroke=stroke,
            stroke_width=width,
        )


class CurvedArrow(VMobject):
    def __init__(
        self,
        start,
        end,
        angle=1.5707963267948966,
        color=WHITE,
        stroke_width=6.0,
        **kwargs,
    ):
        super().__init__(color=color, stroke_width=stroke_width, **kwargs)
        self.start = start
        self.end = end
        self.angle = angle

    def _add(self, raw):
        _, stroke, width = self._style()
        x1, y1 = as_xy(self.start)
        x2, y2 = as_xy(self.end)
        return raw.add_curved_arrow(
            x1, y1, x2, y2, sweep=self.angle, stroke=stroke, stroke_width=width
        )


class Angle(VMobject):
    def __init__(self, vertex, p1, p2, radius=0.4, **kwargs):
        super().__init__(**kwargs)
        self.vertex = vertex
        self.p1 = p1
        self.p2 = p2
        self.radius = radius

    def _add(self, raw):
        _, stroke, width = self._style()
        vx, vy = as_xy(self.vertex)
        x1, y1 = as_xy(self.p1)
        x2, y2 = as_xy(self.p2)
        return raw.add_angle(
            vx,
            vy,
            x1,
            y1,
            x2,
            y2,
            radius=self.radius,
            stroke=stroke,
            stroke_width=width,
        )


class RightAngle(VMobject):
    def __init__(self, vertex, p1, p2, size=0.3, **kwargs):
        super().__init__(**kwargs)
        self.vertex = vertex
        self.p1 = p1
        self.p2 = p2
        self.size = size

    def _add(self, raw):
        _, stroke, width = self._style()
        vx, vy = as_xy(self.vertex)
        x1, y1 = as_xy(self.p1)
        x2, y2 = as_xy(self.p2)
        return raw.add_right_angle(
            vx,
            vy,
            x1,
            y1,
            x2,
            y2,
            size=self.size,
            stroke=stroke,
            stroke_width=width,
        )


class PolarPlane(Mobject):
    def __init__(
        self,
        radius=4.0,
        radius_step=1.0,
        azimuth_divisions=12,
        faded_line_ratio=1,
    ):
        super().__init__()
        self.radius = radius
        self.radius_step = radius_step
        self.azimuth_divisions = azimuth_divisions
        self.faded_line_ratio = faded_line_ratio

    def _add(self, raw):
        return raw.add_polar_plane(
            radius=self.radius,
            radius_step=self.radius_step,
            azimuth_divisions=self.azimuth_divisions,
            faded_line_ratio=self.faded_line_ratio,
        )


class Underline(Mobject):
    def __init__(self, mobject, color=WHITE, buff=0.1):
        super().__init__()
        self.target = mobject
        self.color = color
        self.buff = buff

    def _add(self, raw):
        tid = _node_id(self.target, raw)
        return raw.add_underline(tid, buff=self.buff, stroke=self.color)


class Cross(Mobject):
    def __init__(self, mobject, color=RED):
        super().__init__()
        self.target = mobject
        self.color = color

    def _add(self, raw):
        tid = _node_id(self.target, raw)
        return raw.add_cross(tid, stroke=self.color)


class Code(Mobject):
    def __init__(self, source, color=WHITE, font_size=28.0):
        super().__init__()
        self.source = source
        self.color = color
        self.font_size = font_size

    def _add(self, raw):
        return raw.add_code(self.source, font_size_pt=self.font_size, color=self.color)


class Matrix(Mobject):
    def __init__(self, matrix, font_size=42.0, color=WHITE):
        super().__init__()
        self.matrix = matrix
        self.font_size = font_size
        self.color = color

    def _add(self, raw):
        return raw.add_matrix(self.matrix, font_size_pt=self.font_size, color=self.color)


class Elbow(VMobject):
    def __init__(self, corner, width=0.5, angle=0.0, **kwargs):
        super().__init__(**kwargs)
        self.corner = corner
        self.elbow_width = width
        self.angle = angle

    def _add(self, raw):
        _, stroke, sw = self._style()
        x, y = as_xy(self.corner)
        return raw.add_elbow(
            x, y, size=self.elbow_width, angle=self.angle, stroke=stroke, stroke_width=sw
        )


class CubicBezier(VMobject):
    def __init__(self, start, control1, control2, end, **kwargs):
        super().__init__(**kwargs)
        self.start = start
        self.control1 = control1
        self.control2 = control2
        self.end = end

    def _add(self, raw):
        _, stroke, sw = self._style()
        x0, y0 = as_xy(self.start)
        x1, y1 = as_xy(self.control1)
        x2, y2 = as_xy(self.control2)
        x3, y3 = as_xy(self.end)
        return raw.add_cubic_bezier(
            x0, y0, x1, y1, x2, y2, x3, y3, stroke=stroke, stroke_width=sw
        )


class BraceLabel(Mobject):
    def __init__(self, mobject, label, direction=DOWN, font_size=36.0):
        super().__init__()
        self.target = mobject
        self.label = label
        self.direction = direction
        self.font_size = font_size

    def _add(self, raw):
        tid = _node_id(self.target, raw)
        return raw.add_brace_label(
            tid,
            self.label,
            dir_name(self.direction),
            font_size_pt=self.font_size,
        )
