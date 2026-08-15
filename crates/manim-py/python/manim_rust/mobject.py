"""Deferred CE-style mobjects. Geometry is created on first add/play."""

from __future__ import annotations

from manim_rust.constants import (
    DEFAULT_MOBJECT_TO_MOBJECT_BUFFER,
    DOWN,
    LEFT,
    RIGHT,
    WHITE,
    YELLOW,
    as_xy,
    dir_name,
)


class Mobject:
    def __init__(self):
        self._id = None
        self._raw = None
        self._pending = []

    def _add(self, raw):
        raise NotImplementedError

    def _materialize(self, raw):
        if self._id is not None:
            return self._id
        self._raw = raw
        self._id = self._add(raw)
        for op in self._pending:
            op(raw, self._id)
        self._pending.clear()
        return self._id

    def _apply(self, op):
        if self._id is not None:
            op(self._raw, self._id)
        else:
            self._pending.append(op)
        return self

    def shift(self, delta):
        dx, dy = as_xy(delta)

        def op(raw, node):
            raw.shift(node, dx, dy)

        return self._apply(op)

    def move_to(self, point):
        x, y = as_xy(point)

        def op(raw, node):
            raw.move_to(node, x, y)

        return self._apply(op)

    def next_to(self, other, direction=RIGHT, buff=DEFAULT_MOBJECT_TO_MOBJECT_BUFFER):
        def op(raw, node):
            oid = _node_id(other, raw)
            raw.next_to(node, oid, dir_name(direction), buff)

        return self._apply(op)

    def align_to(self, other, direction=RIGHT):
        def op(raw, node):
            oid = _node_id(other, raw)
            raw.align_to(node, oid, dir_name(direction))

        return self._apply(op)

    def to_edge(self, direction=LEFT, buff=0.5):
        def op(raw, node):
            raw.to_edge(node, dir_name(direction), buff)

        return self._apply(op)

    def to_corner(self, direction="ul", buff=0.5):
        def op(raw, node):
            raw.to_corner(node, dir_name(direction), buff)

        return self._apply(op)

    def scale(self, factor):
        def op(raw, node):
            raw.scale(node, factor)

        return self._apply(op)

    def rotate(self, angle):
        def op(raw, node):
            raw.rotate(node, angle)

        return self._apply(op)

    def set_color(self, color):
        def op(raw, node):
            raw.set_color(node, color)

        return self._apply(op)

    def set_z_index(self, z):
        def op(raw, node):
            raw.set_z_index(node, z)

        return self._apply(op)

    def set_width(self, width):
        def op(raw, node):
            raw.set_width(node, width)

        return self._apply(op)

    def set_height(self, height):
        def op(raw, node):
            raw.set_height(node, height)

        return self._apply(op)

    def set_x(self, x):
        def op(raw, node):
            raw.set_x(node, x)

        return self._apply(op)

    def set_y(self, y):
        def op(raw, node):
            raw.set_y(node, y)

        return self._apply(op)

    @property
    def animate(self):
        return _Animate(self)


def _node_id(obj, raw):
    if isinstance(obj, int):
        return obj
    return obj._materialize(raw)


def _stroke_fill(color, fill_opacity, fill_color=None, stroke_width=4.0):
    fill = None
    if fill_opacity and fill_opacity > 0:
        fill = fill_color or color
    return fill, color, stroke_width


class VMobject(Mobject):
    def __init__(
        self,
        color=WHITE,
        fill_opacity=0.0,
        fill_color=None,
        stroke_width=4.0,
        **kwargs,
    ):
        super().__init__()
        self.color = kwargs.get("stroke_color", color)
        self.fill_opacity = fill_opacity
        self.fill_color = fill_color
        self.stroke_width = stroke_width

    def _style(self):
        return _stroke_fill(
            self.color, self.fill_opacity, self.fill_color, self.stroke_width
        )


class Arc(VMobject):
    def __init__(self, radius=1.0, start_angle=0.0, angle=3.141592653589793, **kwargs):
        super().__init__(**kwargs)
        self.radius = radius
        self.start_angle = start_angle
        self.angle = angle

    def _add(self, raw):
        _, stroke, width = self._style()
        return raw.add_arc(
            radius=self.radius,
            start_angle=self.start_angle,
            sweep=self.angle,
            stroke=stroke,
            stroke_width=width,
        )


class Sector(VMobject):
    def __init__(self, radius=1.0, start_angle=0.0, angle=1.5707963267948966, **kwargs):
        super().__init__(**kwargs)
        self.radius = radius
        self.start_angle = start_angle
        self.angle = angle

    def _add(self, raw):
        fill, stroke, width = self._style()
        return raw.add_sector(
            radius=self.radius,
            start_angle=self.start_angle,
            sweep=self.angle,
            fill=fill,
            stroke=stroke,
            stroke_width=width,
        )


class Annulus(VMobject):
    def __init__(self, inner_radius=0.5, outer_radius=1.0, **kwargs):
        super().__init__(**kwargs)
        self.inner_radius = inner_radius
        self.outer_radius = outer_radius

    def _add(self, raw):
        fill, stroke, width = self._style()
        return raw.add_annulus(
            inner=self.inner_radius,
            outer=self.outer_radius,
            fill=fill,
            stroke=stroke,
            stroke_width=width,
        )


class Circle(VMobject):
    def __init__(self, radius=1.0, **kwargs):
        super().__init__(**kwargs)
        self.radius = radius

    def _add(self, raw):
        fill, stroke, width = self._style()
        return raw.add_circle(
            radius=self.radius, fill=fill, stroke=stroke, stroke_width=width
        )


class Dot(VMobject):
    def __init__(self, point=None, radius=0.08, color=WHITE, fill_opacity=1.0, **kwargs):
        super().__init__(color=color, fill_opacity=fill_opacity, stroke_width=0.0, **kwargs)
        self.point = point or (0.0, 0.0)
        self.radius = radius

    def _add(self, raw):
        fill, stroke, width = self._style()
        x, y = as_xy(self.point)
        return raw.add_dot(
            x=x, y=y, radius=self.radius, fill=fill, stroke=stroke, stroke_width=width
        )


class Square(VMobject):
    def __init__(self, side_length=2.0, **kwargs):
        super().__init__(**kwargs)
        self.side_length = side_length

    def _add(self, raw):
        fill, stroke, width = self._style()
        return raw.add_square(
            side=self.side_length, fill=fill, stroke=stroke, stroke_width=width
        )


class Rectangle(VMobject):
    def __init__(self, width=3.0, height=2.0, **kwargs):
        super().__init__(**kwargs)
        self.width = width
        self.height = height

    def _add(self, raw):
        fill, stroke, width = self._style()
        return raw.add_rect(
            width=self.width,
            height=self.height,
            fill=fill,
            stroke=stroke,
            stroke_width=width,
        )


class Ellipse(VMobject):
    def __init__(self, width=3.0, height=1.6, **kwargs):
        super().__init__(**kwargs)
        self.width = width
        self.height = height

    def _add(self, raw):
        fill, stroke, sw = self._style()
        return raw.add_ellipse(
            rx=self.width / 2.0,
            ry=self.height / 2.0,
            fill=fill,
            stroke=stroke,
            stroke_width=sw,
        )


class Line(VMobject):
    def __init__(self, start, end, color=WHITE, stroke_width=4.0, **kwargs):
        super().__init__(color=color, stroke_width=stroke_width, **kwargs)
        self.start = start
        self.end = end

    def _add(self, raw):
        _, stroke, width = self._style()
        x1, y1 = as_xy(self.start)
        x2, y2 = as_xy(self.end)
        return raw.add_line(x1, y1, x2, y2, stroke=stroke, stroke_width=width)


class Arrow(VMobject):
    def __init__(self, start, end, color=WHITE, stroke_width=6.0, buff=0.25, **kwargs):
        super().__init__(color=color, stroke_width=stroke_width, **kwargs)
        self.start = start
        self.end = end
        self.buff = buff

    def _add(self, raw):
        _, stroke, width = self._style()
        x1, y1 = as_xy(self.start)
        x2, y2 = as_xy(self.end)
        return raw.add_arrow(
            x1, y1, x2, y2, buff=self.buff, stroke=stroke, stroke_width=width
        )


class Star(VMobject):
    def __init__(self, n=5, outer_radius=1.0, inner_radius=None, **kwargs):
        super().__init__(**kwargs)
        self.n = n
        self.outer_radius = outer_radius
        self.inner_radius = inner_radius

    def _add(self, raw):
        fill, stroke, width = self._style()
        return raw.add_star(
            n=self.n,
            outer=self.outer_radius,
            inner=self.inner_radius,
            fill=fill,
            stroke=stroke,
            stroke_width=width,
        )


class RegularPolygon(VMobject):
    def __init__(self, n=6, radius=1.0, **kwargs):
        super().__init__(**kwargs)
        self.n = n
        self.radius = radius

    def _add(self, raw):
        fill, stroke, width = self._style()
        return raw.add_regular_polygon(
            sides=self.n,
            radius=self.radius,
            fill=fill,
            stroke=stroke,
            stroke_width=width,
        )


class Text(Mobject):
    def __init__(self, text, color=WHITE, font_size=48.0):
        super().__init__()
        self.text = text
        self.color = color
        self.font_size = font_size

    def _add(self, raw):
        return raw.add_text(self.text, color=self.color, font_size_pt=self.font_size)


class MathTex(Mobject):
    def __init__(self, source, color=WHITE, font_size=48.0):
        super().__init__()
        self.source = source
        self.color = color
        self.font_size = font_size

    def _add(self, raw):
        return raw.add_tex(
            self.source,
            color=self.color,
            font_size_pt=self.font_size,
            syntax="latex",
        )


class Tex(MathTex):
    pass


class DecimalNumber(Mobject):
    def __init__(self, value=0.0, num_decimal_places=2, color=WHITE, font_size=48.0):
        super().__init__()
        self.value = float(value)
        self.num_decimal_places = num_decimal_places
        self.color = color
        self.font_size = font_size

    def _add(self, raw):
        return raw.add_decimal_atlas(
            self.value,
            self.num_decimal_places,
            color=self.color,
            font_size_pt=self.font_size,
        )

    def set_value(self, value):
        self.value = float(value)
        return self

    @property
    def animate(self):
        return _DecimalAnimate(self)


class ValueTracker:
    """Authoring-time float. Animate a DecimalNumber, not this, for display."""

    def __init__(self, value=0.0):
        self._value = float(value)

    def get_value(self):
        return self._value

    def set_value(self, value):
        self._value = float(value)
        return self


class DashedVMobject(Mobject):
    def __init__(self, vmobject, num_dashes=15, dashed_ratio=0.5):
        super().__init__()
        self.vmobject = vmobject
        self.num_dashes = num_dashes
        self.dashed_ratio = dashed_ratio

    def _add(self, raw):
        src = self.vmobject._materialize(raw)
        return raw.add_dashed_copy(src, self.num_dashes, self.dashed_ratio)


class Title(Mobject):
    def __init__(self, source, color=WHITE, font_size=48.0):
        super().__init__()
        self.source = source
        self.color = color
        self.font_size = font_size

    def _add(self, raw):
        return raw.add_title(self.source, color=self.color, font_size_pt=self.font_size)


class NumberPlane(Mobject):
    def __init__(self, x_range=(-7.0, 7.0), y_range=(-4.0, 4.0), faded_line_ratio=1):
        super().__init__()
        self.x_range = x_range
        self.y_range = y_range
        self.faded_line_ratio = faded_line_ratio

    def _add(self, raw):
        return raw.add_number_plane(
            x_min=self.x_range[0],
            x_max=self.x_range[1],
            y_min=self.y_range[0],
            y_max=self.y_range[1],
            faded_line_ratio=self.faded_line_ratio,
        )


class NumberLine(Mobject):
    def __init__(
        self,
        x_range=(-4.0, 4.0, 1.0),
        include_tip=False,
        include_numbers=False,
        color=WHITE,
    ):
        super().__init__()
        self.x_range = x_range
        self.include_tip = include_tip
        self.include_numbers = include_numbers
        self.color = color

    def _add(self, raw):
        nid = raw.add_number_line(
            x_min=self.x_range[0],
            x_max=self.x_range[1],
            x_step=self.x_range[2] if len(self.x_range) > 2 else 1.0,
            include_tip=self.include_tip,
            stroke=self.color,
        )
        if self.include_numbers:
            raw.add_number_line_labels(
                x_min=self.x_range[0],
                x_max=self.x_range[1],
                x_step=self.x_range[2] if len(self.x_range) > 2 else 1.0,
                include_tip=self.include_tip,
            )
        return nid


class ComplexPlane(Mobject):
    def __init__(self, x_range=(-7.0, 7.0), y_range=(-4.0, 4.0), include_numbers=False):
        super().__init__()
        self.x_range = x_range
        self.y_range = y_range
        self.include_numbers = include_numbers

    def _add(self, raw):
        nid = raw.add_complex_plane(
            x_min=self.x_range[0],
            x_max=self.x_range[1],
            y_min=self.y_range[0],
            y_max=self.y_range[1],
        )
        if self.include_numbers:
            raw.add_complex_plane_labels(
                x_min=self.x_range[0],
                x_max=self.x_range[1],
                y_min=self.y_range[0],
                y_max=self.y_range[1],
            )
        return nid


class Brace(Mobject):
    def __init__(self, mobject, direction=DOWN, color=WHITE):
        super().__init__()
        self.target = mobject
        self.direction = direction
        self.color = color

    def _add(self, raw):
        tid = _node_id(self.target, raw)
        return raw.add_brace(tid, dir_name(self.direction), stroke=self.color)


class SurroundingRectangle(Mobject):
    def __init__(self, mobject, color=YELLOW, buff=0.15):
        super().__init__()
        self.target = mobject
        self.color = color
        self.buff = buff

    def _add(self, raw):
        tid = _node_id(self.target, raw)
        return raw.add_surrounding_rect(tid, buff=self.buff, stroke=self.color)


class Axes(Mobject):
    def __init__(self, x_range=(-3.0, 3.0), y_range=(-2.0, 2.0)):
        super().__init__()
        self.x_range = x_range
        self.y_range = y_range

    def _add(self, raw):
        return raw.add_axes(
            x_min=self.x_range[0],
            x_max=self.x_range[1],
            y_min=self.y_range[0],
            y_max=self.y_range[1],
        )


class Table(Mobject):
    def __init__(self, cells, font_size=36.0, color=WHITE):
        super().__init__()
        self.cells = cells
        self.font_size = font_size
        self.color = color

    def _add(self, raw):
        return raw.add_table(self.cells, font_size_pt=self.font_size, color=self.color)


class ArrowVectorField(Mobject):
    def __init__(
        self,
        func,
        x_range=(-3.0, 3.0, 1.0),
        y_range=(-2.0, 2.0, 1.0),
        max_len=0.45,
        color=YELLOW,
    ):
        super().__init__()
        self.func = func
        self.x_range = x_range
        self.y_range = y_range
        self.max_len = max_len
        self.color = color

    def _add(self, raw):
        def vx(x, y):
            return float(self.func(x, y)[0])

        def vy(x, y):
            return float(self.func(x, y)[1])

        return raw.add_arrow_field(
            vx,
            vy,
            x_min=self.x_range[0],
            x_max=self.x_range[1],
            y_min=self.y_range[0],
            y_max=self.y_range[1],
            x_step=self.x_range[2] if len(self.x_range) > 2 else 1.0,
            y_step=self.y_range[2] if len(self.y_range) > 2 else 1.0,
            max_len=self.max_len,
            stroke=self.color,
        )


class VGroup(Mobject):
    def __init__(self, *children):
        super().__init__()
        self.submobjects = list(children)

    def add(self, *children):
        self.submobjects.extend(children)
        return self

    def _add(self, raw):
        ids = [child._materialize(raw) for child in self.submobjects]
        return raw.add_group(ids)

    def arrange(self, direction=RIGHT, buff=DEFAULT_MOBJECT_TO_MOBJECT_BUFFER, center=True):
        def op(raw, node):
            raw.arrange(node, dir_name(direction), buff, center)

        return self._apply(op)

    def arrange_in_grid(self, rows=None, cols=None, buff_x=0.25, buff_y=0.25, center=True):
        def op(raw, node):
            raw.arrange_in_grid(node, rows, cols, buff_x, buff_y, center)

        return self._apply(op)

    def __iter__(self):
        return iter(self.submobjects)

    def __len__(self):
        return len(self.submobjects)

    def __getitem__(self, i):
        return self.submobjects[i]


class _Animate:
    def __init__(self, mobject, run_time=1.0, rate_func="smooth"):
        self.mobject = mobject
        self.run_time = run_time
        self.rate_func = rate_func

    def __call__(self, run_time=1.0, rate_func="smooth"):
        return _Animate(self.mobject, run_time=run_time, rate_func=rate_func)

    def shift(self, delta):
        from manim_rust.animation import _BoundAnim

        return _BoundAnim(self.mobject, "shift", self.run_time, self.rate_func, delta=delta)

    def scale(self, factor):
        from manim_rust.animation import _BoundAnim

        return _BoundAnim(self.mobject, "scale", self.run_time, self.rate_func, factor=factor)

    def rotate(self, angle):
        from manim_rust.animation import _BoundAnim

        return _BoundAnim(self.mobject, "rotate", self.run_time, self.rate_func, angle=angle)

    def next_to(self, other, direction=RIGHT, buff=DEFAULT_MOBJECT_TO_MOBJECT_BUFFER):
        from manim_rust.animation import _BoundAnim

        return _BoundAnim(
            self.mobject,
            "next_to",
            self.run_time,
            self.rate_func,
            other=other,
            direction=direction,
            buff=buff,
        )

    def move_to(self, point):
        from manim_rust.animation import _BoundAnim

        return _BoundAnim(self.mobject, "move_to", self.run_time, self.rate_func, point=point)

    def set_color(self, color):
        from manim_rust.animation import _BoundAnim

        return _BoundAnim(self.mobject, "recolor", self.run_time, self.rate_func, color=color)


class _DecimalAnimate(_Animate):
    def set_value(self, value):
        from manim_rust.animation import _BoundAnim

        return _BoundAnim(
            self.mobject,
            "changing_decimal",
            self.run_time,
            self.rate_func,
            from_value=self.mobject.value,
            to_value=float(value),
            places=self.mobject.num_decimal_places,
        )
