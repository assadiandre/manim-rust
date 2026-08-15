"""Deferred CE-style mobjects. Geometry is created on first add/play."""

from __future__ import annotations

from manim_rust.constants import (
    BLUE,
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

    def set_opacity(self, opacity):
        def op(raw, node):
            raw.set_opacity(node, float(opacity))

        return self._apply(op)

    def generate_target(self):
        self.target = _Target()
        return self.target

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


class ArcBetweenPoints(VMobject):
    def __init__(self, start, end, angle=1.5707963267948966, **kwargs):
        super().__init__(**kwargs)
        self.start = start
        self.end = end
        self.angle = angle

    def _add(self, raw):
        _, stroke, width = self._style()
        x1, y1 = as_xy(self.start)
        x2, y2 = as_xy(self.end)
        return raw.add_arc_between_points(
            x1, y1, x2, y2, sweep=self.angle, stroke=stroke, stroke_width=width
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


class AnnularSector(VMobject):
    def __init__(
        self,
        inner_radius=0.5,
        outer_radius=1.0,
        start_angle=0.0,
        angle=1.5707963267948966,
        **kwargs,
    ):
        super().__init__(**kwargs)
        self.inner_radius = inner_radius
        self.outer_radius = outer_radius
        self.start_angle = start_angle
        self.angle = angle

    def _add(self, raw):
        fill, stroke, width = self._style()
        return raw.add_annular_sector(
            inner=self.inner_radius,
            outer=self.outer_radius,
            start_angle=self.start_angle,
            sweep=self.angle,
            fill=fill,
            stroke=stroke,
            stroke_width=width,
        )


class Triangle(VMobject):
    def __init__(self, radius=1.0, **kwargs):
        super().__init__(**kwargs)
        self.radius = radius

    def _add(self, raw):
        fill, stroke, width = self._style()
        return raw.add_triangle(radius=self.radius, fill=fill, stroke=stroke, stroke_width=width)


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


class ScreenRectangle(Rectangle):
    """16:9 rectangle (Manim `ScreenRectangle`)."""

    def __init__(self, aspect_ratio=16.0 / 9.0, height=4.0, **kwargs):
        super().__init__(width=aspect_ratio * height, height=height, **kwargs)


class FullScreenRectangle(Rectangle):
    """Frame-filling rectangle (Manim `FullScreenRectangle`)."""

    def __init__(self, **kwargs):
        super().__init__(width=8.0 * 16.0 / 9.0, height=8.0, **kwargs)


class RoundedRectangle(Rectangle):
    def __init__(self, width=3.0, height=2.0, corner_radius=0.25, **kwargs):
        super().__init__(width=width, height=height, **kwargs)
        self.corner_radius = corner_radius

    def _add(self, raw):
        fill, stroke, width = self._style()
        return raw.add_rounded_rect(
            width=self.width,
            height=self.height,
            corner_radius=self.corner_radius,
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


class MarkupText(Mobject):
    def __init__(self, text, color=WHITE, font_size=48.0):
        super().__init__()
        self.text = text
        self.color = color
        self.font_size = font_size

    def _add(self, raw):
        return raw.add_markup(self.text, color=self.color, font_size_pt=self.font_size)


class Paragraph(Mobject):
    def __init__(self, *text, line_spacing=-1.0, alignment=None, color=WHITE, font_size=48.0):
        super().__init__()
        self.text = "\n".join(text)
        self.line_spacing = line_spacing
        self.alignment = alignment
        self.color = color
        self.font_size = font_size

    def _add(self, raw):
        return raw.add_paragraph(
            self.text,
            line_spacing=self.line_spacing,
            alignment=self.alignment,
            color=self.color,
            font_size_pt=self.font_size,
        )


class _BoundMobject(Mobject):
    """Already-materialized node (MathTex parts, graph vertices)."""

    def __init__(self, raw, node_id):
        super().__init__()
        self._raw = raw
        self._id = node_id

    def _add(self, raw):
        return self._id


class _DeferredTexPart(Mobject):
    """`eq[i]` before the parent MathTex is added to a scene."""

    def __init__(self, parent, index):
        super().__init__()
        self._parent = parent
        self._index = index

    def _add(self, raw):
        pid = self._parent._materialize(raw)
        kids = raw.children_of(pid)
        self._id = kids[self._index] if kids else pid
        self._raw = raw
        return self._id

    def set_color(self, color):
        idx = self._index

        def op(raw, node):
            kids = raw.children_of(node)
            raw.set_color(kids[idx] if kids else node, color)

        return self._parent._apply(op)


class MathTex(Mobject):
    def __init__(self, *tex_strings, color=WHITE, font_size=48.0):
        super().__init__()
        if not tex_strings:
            tex_strings = ("",)
        self.tex_strings = [str(s) for s in tex_strings]
        self.source = "".join(self.tex_strings)
        self.color = color
        self.font_size = font_size
        self.submobjects = []

    def _add(self, raw):
        nid = raw.add_tex_parts(
            self.tex_strings,
            color=self.color,
            font_size_pt=self.font_size,
            syntax="latex",
        )
        kids = raw.children_of(nid)
        self.submobjects = [_BoundMobject(raw, cid) for cid in kids]
        if not self.submobjects:
            self.submobjects = [_BoundMobject(raw, nid)]
        return nid

    def __getitem__(self, i):
        if self._id is None:
            return _DeferredTexPart(self, i)
        return self.submobjects[i]

    def __len__(self):
        return len(self.tex_strings)

    def set_color_by_tex(self, tex, color):
        def op(raw, node):
            raw.set_color_by_tex(node, tex, color)

        return self._apply(op)

    def get_part_by_tex(self, tex):
        if self._id is None:
            raise RuntimeError("get_part_by_tex requires the mobject to be added first")
        pid = self._raw.part_by_tex(self._id, tex)
        if pid is None:
            return None
        return _BoundMobject(self._raw, pid)


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


class Integer(DecimalNumber):
    def __init__(self, value=0, color=WHITE, font_size=48.0):
        super().__init__(value=value, num_decimal_places=0, color=color, font_size=font_size)


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


class BackgroundRectangle(Mobject):
    def __init__(self, mobject, color="black", buff=0.15, fill_opacity=0.75):
        super().__init__()
        self.target = mobject
        self.color = color
        self.buff = buff
        self.fill_opacity = fill_opacity

    def _add(self, raw):
        tid = _node_id(self.target, raw)
        return raw.add_background_rect(
            tid, buff=self.buff, fill=self.color, opacity=self.fill_opacity
        )


class GraphLabel(Mobject):
    def __init__(
        self,
        plot,
        source,
        x=1.0,
        direction=RIGHT,
        buff=0.25,
        color=WHITE,
        font_size=36.0,
    ):
        super().__init__()
        self.plot = plot
        self.source = source
        self.x = x
        self.direction = direction
        self.buff = buff
        self.color = color
        self.font_size = font_size

    def _add(self, raw):
        return raw.add_graph_label(
            _node_id(self.plot, raw),
            self.source,
            x=self.x,
            direction=dir_name(self.direction),
            buff=self.buff,
            color=self.color,
            font_size_pt=self.font_size,
        )


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

    def plot(self, func, x_range=None, color=YELLOW, stroke_width=4.0):
        xr = x_range or self.x_range
        return _AxesPlot(
            self,
            func,
            xr[0],
            xr[1],
            color=color,
            stroke_width=stroke_width,
            fill_opacity=0.0,
        )

    def get_graph_label(self, graph, label, x=1.0, direction=RIGHT, buff=0.25, **kwargs):
        return GraphLabel(graph, label, x=x, direction=direction, buff=buff, **kwargs)

    def get_area(self, graph, x_range=None, color=BLUE, opacity=0.4):
        xr = x_range or getattr(graph, "x_range", None) or self.x_range
        return _AxesArea(graph, xr[0], xr[1], color=color, opacity=opacity)

    def get_riemann_rectangles(self, graph, x_range=None, dx=0.5, color=BLUE, fill_opacity=0.75):
        xr = x_range or getattr(graph, "x_range", None) or self.x_range
        n = max(1, int(round((xr[1] - xr[0]) / dx)))
        return _AxesRiemann(graph, xr[0], xr[1], n, color=color, opacity=fill_opacity)

    def get_tangent_line(self, graph, x, length=2.0, **kwargs):
        return TangentLine(graph, x=x, length=length, **kwargs)

    def get_vertical_line_to_graph(self, x, graph, line_func=None, **kwargs):
        del line_func
        return VerticalLineToGraph(graph, x=x, **kwargs)


class Table(Mobject):
    def __init__(
        self,
        cells,
        font_size=36.0,
        color=WHITE,
        include_outer_lines=False,
        include_inner_lines=True,
        h_buff=0.25,
        v_buff=0.25,
        line_color=WHITE,
        line_stroke_width=2.0,
        row_labels=None,
        col_labels=None,
        top_left_entry=None,
    ):
        super().__init__()
        self.cells = cells
        self.font_size = font_size
        self.color = color
        self.include_outer_lines = include_outer_lines
        self.include_inner_lines = include_inner_lines
        self.h_buff = h_buff
        self.v_buff = v_buff
        self.line_color = line_color
        self.line_stroke_width = line_stroke_width
        self.row_labels = [str(x) for x in row_labels] if row_labels else []
        self.col_labels = [str(x) for x in col_labels] if col_labels else []
        self.top_left_entry = "" if top_left_entry is None else str(top_left_entry)

    def _grid_cols(self):
        data_cols = max((len(r) for r in self.cells), default=0)
        return data_cols + (1 if self.row_labels else 0)

    def add_highlighted_cell(self, pos, color=YELLOW, opacity=0.45):
        row, col = int(pos[0]) - 1, int(pos[1]) - 1
        index = row * self._grid_cols() + col

        def op(raw, node):
            raw.add_highlighted_cell(node, index, color=color, opacity=opacity)

        return self._apply(op)

    def _add(self, raw):
        return raw.add_table(
            self.cells,
            font_size_pt=self.font_size,
            color=self.color,
            include_inner_lines=self.include_inner_lines,
            include_outer_lines=self.include_outer_lines,
            buff_x=self.h_buff,
            buff_y=self.v_buff,
            line_color=self.line_color,
            line_stroke_width=self.line_stroke_width,
            row_labels=self.row_labels,
            col_labels=self.col_labels,
            top_left=self.top_left_entry,
        )


class MathTable(Table):
    def _add(self, raw):
        cells = [[str(c) for c in row] for row in self.cells]
        return raw.add_math_table(
            cells,
            font_size_pt=self.font_size,
            color=self.color,
            include_inner_lines=self.include_inner_lines,
            include_outer_lines=self.include_outer_lines,
            buff_x=self.h_buff,
            buff_y=self.v_buff,
            line_color=self.line_color,
            line_stroke_width=self.line_stroke_width,
        )


class IntegerTable(MathTable):
    def __init__(self, cells, **kwargs):
        formatted = []
        for row in cells:
            formatted.append(
                [str(int(c)) if not isinstance(c, str) else c for c in row]
            )
        super().__init__(formatted, **kwargs)


class BulletedList(Mobject):
    def __init__(self, *items, buff=0.5, color=WHITE, font_size=42.0):
        super().__init__()
        self.items = items
        self.buff = buff
        self.color = color
        self.font_size = font_size

    def _add(self, raw):
        return raw.add_bulleted_list(
            list(self.items),
            buff=self.buff,
            color=self.color,
            font_size_pt=self.font_size,
        )


class NumberedList(Mobject):
    def __init__(self, *items, buff=0.5, color=WHITE, font_size=42.0):
        super().__init__()
        self.items = items
        self.buff = buff
        self.color = color
        self.font_size = font_size

    def _add(self, raw):
        return raw.add_numbered_list(
            list(self.items),
            buff=self.buff,
            color=self.color,
            font_size_pt=self.font_size,
        )


class FunctionGraph(VMobject):
    def __init__(self, function, x_range=(-7.0, 7.0), color=YELLOW, stroke_width=4.0, **kwargs):
        super().__init__(color=color, stroke_width=stroke_width, fill_opacity=0.0, **kwargs)
        self.function = function
        self.x_range = x_range

    def _add(self, raw):
        _, stroke, width = self._style()
        xr = self.x_range
        return raw.add_plot(
            self.function,
            xr[0],
            xr[1],
            samples=int(xr[2]) if len(xr) > 2 else 80,
            stroke=stroke,
            stroke_width=width,
        )


class ParametricFunction(VMobject):
    def __init__(self, function, t_range=(0.0, 1.0), color=YELLOW, stroke_width=4.0, **kwargs):
        super().__init__(color=color, stroke_width=stroke_width, fill_opacity=0.0, **kwargs)
        self.function = function
        self.t_range = t_range

    def _add(self, raw):
        _, stroke, width = self._style()
        tr = self.t_range
        samples = 120
        if len(tr) > 2 and tr[2] > 0:
            samples = max(2, int(round((tr[1] - tr[0]) / tr[2])) + 1)
        return raw.add_parametric(
            self.function,
            tr[0],
            tr[1],
            samples=samples,
            stroke=stroke,
            stroke_width=width,
        )


class BarChart(Mobject):
    def __init__(
        self,
        values,
        bar_names=None,
        y_range=None,
        x_length=6.0,
        y_length=4.0,
        bar_colors=None,
        bar_width=0.6,
        bar_fill_opacity=0.75,
        bar_stroke_width=2.0,
        font_size=28.0,
    ):
        super().__init__()
        self.values = [float(v) for v in values]
        self.bar_names = list(bar_names) if bar_names is not None else []
        if y_range is None:
            lo = min([0.0] + self.values)
            hi = max([0.0] + self.values)
            if hi <= lo:
                hi = lo + 1.0
            self.y_range = (lo, hi)
        else:
            self.y_range = y_range
        self.x_length = x_length
        self.y_length = y_length
        self.bar_colors = list(bar_colors) if bar_colors is not None else []
        self.bar_width = bar_width
        self.bar_fill_opacity = bar_fill_opacity
        self.bar_stroke_width = bar_stroke_width
        self.font_size = font_size

    def _add(self, raw):
        return raw.add_bar_chart(
            self.values,
            names=self.bar_names,
            y_min=self.y_range[0],
            y_max=self.y_range[1],
            x_length=self.x_length,
            y_length=self.y_length,
            bar_width=self.bar_width,
            colors=self.bar_colors,
            fill_opacity=self.bar_fill_opacity,
            stroke_width=self.bar_stroke_width,
            font_size_pt=self.font_size,
        )


class Graph(Mobject):
    """Network graph. Layout is baked at authoring time (no NetworkX)."""

    def __init__(
        self,
        vertices,
        edges,
        layout="circular",
        labels=False,
        layout_scale=2.5,
        vertex_radius=0.16,
        directed=False,
        vertex_color=BLUE,
        edge_color=WHITE,
        **kwargs,
    ):
        super().__init__()
        self.vertices = list(vertices)
        self.edges = list(edges)
        self.layout = layout
        self.labels = labels
        self.layout_scale = layout_scale
        self.vertex_radius = vertex_radius
        self.directed = directed
        self.vertex_color = vertex_color
        self.edge_color = edge_color
        self.vertex_config = kwargs.get("vertex_config") or {}
        self.edge_config = kwargs.get("edge_config") or {}

    def _add(self, raw):
        verts = [str(v) for v in self.vertices]
        edges = [(str(a), str(b)) for a, b in self.edges]
        return raw.add_graph(
            verts,
            edges,
            layout=self.layout,
            labels=self.labels,
            layout_scale=self.layout_scale,
            vertex_radius=self.vertex_radius,
            directed=self.directed,
            vertex_color=self.vertex_color,
            edge_color=self.edge_color,
        )


class DiGraph(Graph):
    def __init__(self, vertices, edges, **kwargs):
        kwargs["directed"] = True
        super().__init__(vertices, edges, **kwargs)


class TangentLine(VMobject):
    def __init__(self, mobject, x=1.0, length=2.0, **kwargs):
        super().__init__(**kwargs)
        self.target = mobject
        self.x = x
        self.length = length

    def _add(self, raw):
        _, stroke, width = self._style()
        return raw.add_tangent_line(
            _node_id(self.target, raw),
            self.x,
            self.length,
            stroke=stroke,
            stroke_width=width,
        )


class VerticalLineToGraph(VMobject):
    def __init__(self, mobject, x=1.0, y0=0.0, **kwargs):
        super().__init__(**kwargs)
        self.target = mobject
        self.x = x
        self.y0 = y0

    def _add(self, raw):
        _, stroke, width = self._style()
        return raw.add_vertical_line_to_graph(
            _node_id(self.target, raw),
            self.x,
            self.y0,
            stroke=stroke,
            stroke_width=width,
        )


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


class _AxesPlot(VMobject):
    def __init__(self, axes, func, x_min, x_max, **kwargs):
        super().__init__(**kwargs)
        self.axes = axes
        self.func = func
        self.x_range = (x_min, x_max)
        self.x_min = x_min
        self.x_max = x_max

    def _add(self, raw):
        _, stroke, width = self._style()
        parent = _node_id(self.axes, raw)
        return raw.add_plot(
            self.func,
            self.x_min,
            self.x_max,
            stroke=stroke,
            stroke_width=width,
            parent=parent,
        )


class _AxesArea(Mobject):
    def __init__(self, graph, x_min, x_max, color=BLUE, opacity=0.4):
        super().__init__()
        self.graph = graph
        self.x_min = x_min
        self.x_max = x_max
        self.color = color
        self.opacity = opacity

    def _add(self, raw):
        return raw.add_area(
            self.graph.func,
            self.x_min,
            self.x_max,
            fill=self.color,
            opacity=self.opacity,
        )


class _AxesRiemann(Mobject):
    def __init__(self, graph, x_min, x_max, n, color=BLUE, opacity=0.75):
        super().__init__()
        self.graph = graph
        self.x_min = x_min
        self.x_max = x_max
        self.n = n
        self.color = color
        self.opacity = opacity

    def _add(self, raw):
        return raw.add_riemann(
            self.graph.func,
            self.x_min,
            self.x_max,
            n=self.n,
            color_a=self.color,
            color_b=self.color,
            opacity=self.opacity,
        )


class _Target:
    """Records shift/move_to for `MoveToTarget` (authoring-time only)."""

    def __init__(self):
        self.delta = None
        self.point = None

    def shift(self, delta):
        x, y = as_xy(delta)
        dx, dy = self.delta or (0.0, 0.0)
        self.delta = (dx + x, dy + y)
        return self

    def move_to(self, point):
        self.point = point
        return self


class LabeledDot(Mobject):
    def __init__(self, label, point=None, direction=None, buff=0.15, color=WHITE, font_size=36.0):
        super().__init__()
        self.label = label
        self.point = point or (0.0, 0.0)
        self.direction = direction if direction is not None else (0.0, 1.0)
        self.buff = buff
        self.color = color
        self.font_size = font_size

    def _add(self, raw):
        x, y = as_xy(self.point)
        return raw.add_labeled_dot(
            x,
            y,
            self.label,
            direction=dir_name(self.direction),
            buff=self.buff,
            color=self.color,
            font_size_pt=self.font_size,
        )


class LabeledLine(Mobject):
    def __init__(self, label, start, end, direction=None, buff=0.15, color=WHITE, font_size=36.0):
        super().__init__()
        self.label = label
        self.start = start
        self.end = end
        self.direction = direction if direction is not None else (0.0, 1.0)
        self.buff = buff
        self.color = color
        self.font_size = font_size

    def _add(self, raw):
        x1, y1 = as_xy(self.start)
        x2, y2 = as_xy(self.end)
        return raw.add_labeled_line(
            x1,
            y1,
            x2,
            y2,
            self.label,
            direction=dir_name(self.direction),
            buff=self.buff,
            color=self.color,
            font_size_pt=self.font_size,
        )


class LabeledArrow(Mobject):
    def __init__(self, label, start, end, direction=None, buff=0.15, color=WHITE, font_size=36.0):
        super().__init__()
        self.label = label
        self.start = start
        self.end = end
        self.direction = direction if direction is not None else (0.0, 1.0)
        self.buff = buff
        self.color = color
        self.font_size = font_size

    def _add(self, raw):
        x1, y1 = as_xy(self.start)
        x2, y2 = as_xy(self.end)
        return raw.add_labeled_arrow(
            x1,
            y1,
            x2,
            y2,
            self.label,
            direction=dir_name(self.direction),
            buff=self.buff,
            color=self.color,
            font_size_pt=self.font_size,
        )


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
