"""CE-style Scene: subclass + construct(), or the procedural NodeId API."""

from __future__ import annotations

from manim_rust.animation import Succession
from manim_rust.mobject import Mobject


class Scene:
    def __init__(self, width=1920, height=1080, background="black"):
        from manim_rust._native import Scene as NativeScene

        self._raw = NativeScene(width, height, background)
        self._constructed = False

    def __getattr__(self, name):
        return getattr(self._raw, name)

    def construct(self):
        """Override in a subclass. Called once by render()/save_png()."""

    def _ensure_construct(self):
        if self._constructed:
            return
        self._constructed = True
        if type(self).construct is not Scene.construct:
            self.construct()

    def add(self, *mobjects):
        ids = []
        for mob in mobjects:
            if isinstance(mob, Mobject):
                ids.append(mob._materialize(self._raw))
            else:
                ids.append(mob)
        if len(ids) == 1:
            return ids[0]
        return ids

    def play(self, *anims, run_time=None, rate_func=None, duration=None):
        if duration is not None and run_time is None:
            run_time = duration
        if len(anims) == 1 and isinstance(anims[0], Succession):
            succ = anims[0]
            for child in succ.anims:
                self.play(child, run_time=run_time or succ.run_time, rate_func=rate_func or succ.rate_func)
            return
        specs = []
        for anim in anims:
            if not hasattr(anim, "_spec"):
                raise TypeError(
                    f"Scene.play expected an Animation (Create, FadeIn, "
                    f"mobject.animate...), got {type(anim)!r}"
                )
            specs.append(anim._spec(self._raw, run_time=run_time, rate_func=rate_func))
        if specs:
            self._raw.play_bundle(specs)

    def wait(self, duration=1.0):
        self._raw.wait(duration)

    def render(self, path, fps=60):
        self._ensure_construct()
        return self._raw.render(path, fps)

    def save_png(self, path, time=0.0):
        self._ensure_construct()
        return self._raw.save_png(path, time)

    def duration(self):
        return self._raw.duration()
