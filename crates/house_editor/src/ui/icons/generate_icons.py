#!/usr/bin/env python3
"""Draws the editor's icons as white line art on a transparent background, so the editor can
tint them. Run from this directory to regenerate every PNG: `python3 generate_icons.py`."""

from PIL import Image, ImageDraw

SIZE = 96
SUPERSAMPLE = 4
STROKE = 5


class Canvas:
    def __init__(self):
        self.image = Image.new("RGBA", (SIZE * SUPERSAMPLE,) * 2, (0, 0, 0, 0))
        self.draw = ImageDraw.Draw(self.image)

    def scaled(self, value):
        return value * SUPERSAMPLE

    def line(self, points, width=STROKE):
        points = [(self.scaled(x), self.scaled(y)) for x, y in points]
        w = self.scaled(width)
        self.draw.line(points, fill="white", width=int(w), joint="curve")
        for x, y in (points[0], points[-1]):
            self.draw.ellipse((x - w / 2, y - w / 2, x + w / 2, y + w / 2), fill="white")

    def rect(self, x0, y0, x1, y1, width=STROKE, radius=4, filled=False):
        box = [self.scaled(v) for v in (x0, y0, x1, y1)]
        self.draw.rounded_rectangle(
            box,
            radius=self.scaled(radius),
            outline="white",
            width=int(self.scaled(width)),
            fill="white" if filled else None,
        )

    def ellipse(self, x0, y0, x1, y1, width=STROKE, filled=False):
        box = [self.scaled(v) for v in (x0, y0, x1, y1)]
        self.draw.ellipse(
            box, outline="white", width=int(self.scaled(width)), fill="white" if filled else None
        )

    def arc(self, x0, y0, x1, y1, start, end, width=STROKE):
        box = [self.scaled(v) for v in (x0, y0, x1, y1)]
        self.draw.arc(box, start, end, fill="white", width=int(self.scaled(width)))

    def polygon(self, points, filled=True, width=STROKE):
        if filled:
            self.draw.polygon([(self.scaled(x), self.scaled(y)) for x, y in points], fill="white")
        else:
            self.line(points + [points[0]], width)

    def save(self, name):
        self.image.resize((SIZE, SIZE), Image.LANCZOS).save(f"{name}.png")


ICONS = {}


def icon(function):
    ICONS[function.__name__] = function
    return function


# --- tools ---------------------------------------------------------------------------------


@icon
def select(c):
    c.polygon([(26, 14), (26, 72), (40, 58), (50, 80), (58, 76), (48, 55), (68, 54)], filled=False)


@icon
def wall(c):
    c.polygon([(14, 40), (40, 26), (82, 40), (56, 54)], filled=False)
    c.line([(14, 40), (14, 58), (56, 72), (56, 54)])
    c.line([(56, 72), (82, 58), (82, 40)])


@icon
def curve(c):
    c.arc(8, 20, 88, 100, 200, 340)
    c.line([(14, 62), (14, 78)])
    c.line([(82, 62), (82, 78)])
    c.line([(14, 78), (82, 78)], width=3)


@icon
def door(c):
    c.rect(26, 12, 70, 84)
    c.ellipse(55, 46, 61, 52, filled=True, width=2)


@icon
def window(c):
    c.rect(16, 18, 80, 78)
    c.line([(48, 18), (48, 78)], width=4)
    c.line([(16, 48), (80, 48)], width=4)


@icon
def furniture(c):
    c.rect(14, 38, 82, 66)
    c.rect(22, 26, 74, 42)
    c.line([(24, 66), (24, 78)])
    c.line([(72, 66), (72, 78)])


@icon
def delete(c):
    c.line([(22, 30), (74, 30)])
    c.line([(38, 30), (38, 20), (58, 20), (58, 30)])
    c.rect(28, 30, 68, 80)
    c.line([(42, 42), (42, 68)], width=4)
    c.line([(54, 42), (54, 68)], width=4)


# --- actions -------------------------------------------------------------------------------


@icon
def undo(c):
    c.arc(22, 26, 78, 82, 180, 360 + 80)
    c.line([(22, 54), (22, 30)])
    c.line([(22, 54), (46, 54)])


@icon
def redo(c):
    c.arc(18, 26, 74, 82, 180 - 80, 360)
    c.line([(74, 54), (74, 30)])
    c.line([(74, 54), (50, 54)])


@icon
def save(c):
    c.rect(16, 16, 80, 80)
    c.rect(30, 16, 60, 38, width=4, radius=2)
    c.rect(28, 50, 68, 80, width=4, radius=2)


@icon
def home(c):
    c.line([(12, 46), (48, 14), (84, 46)])
    c.line([(22, 40), (22, 80), (74, 80), (74, 40)])
    c.rect(40, 56, 56, 80, width=4, radius=2)


@icon
def grid(c):
    c.rect(16, 16, 80, 80)
    c.line([(37, 16), (37, 80)], width=4)
    c.line([(59, 16), (59, 80)], width=4)
    c.line([(16, 37), (80, 37)], width=4)
    c.line([(16, 59), (80, 59)], width=4)


@icon
def houses(c):
    c.rect(12, 30, 52, 80, width=4, radius=2)
    c.rect(44, 18, 84, 68, width=4, radius=2)
    c.line([(54, 34), (74, 34)], width=4)
    c.line([(54, 46), (74, 46)], width=4)


@icon
def plus(c):
    c.line([(48, 20), (48, 76)], width=7)
    c.line([(20, 48), (76, 48)], width=7)


@icon
def minus(c):
    c.line([(20, 48), (76, 48)], width=7)


@icon
def rename(c):
    c.polygon([(60, 18), (78, 36), (36, 78), (16, 80), (18, 60)], filled=False)
    c.line([(52, 26), (70, 44)], width=4)


@icon
def duplicate(c):
    c.rect(32, 32, 80, 80)
    c.line([(60, 16), (16, 16), (16, 60)])


@icon
def close(c):
    c.line([(24, 24), (72, 72)], width=7)
    c.line([(72, 24), (24, 72)], width=7)


@icon
def info(c):
    c.ellipse(14, 14, 82, 82)
    c.line([(48, 42), (48, 64)])
    c.ellipse(45, 28, 51, 34, width=2, filled=True)


# --- doors ---------------------------------------------------------------------------------


def threshold(c):
    c.line([(12, 84), (84, 84)], width=4)


@icon
def door_standard(c):
    c.rect(28, 12, 68, 80)
    c.ellipse(55, 44, 61, 50, filled=True, width=2)
    threshold(c)


@icon
def door_arched(c):
    c.line([(28, 80), (28, 40)])
    c.line([(68, 80), (68, 40)])
    c.arc(28, 12, 68, 68, 180, 360)
    c.ellipse(55, 52, 61, 58, filled=True, width=2)
    threshold(c)


@icon
def door_double(c):
    c.rect(14, 12, 82, 80)
    c.line([(48, 12), (48, 80)], width=4)
    c.ellipse(40, 44, 44, 48, filled=True, width=2)
    c.ellipse(52, 44, 56, 48, filled=True, width=2)
    threshold(c)


@icon
def door_glass(c):
    c.rect(28, 12, 68, 80)
    c.rect(37, 21, 59, 54, width=3, radius=2)
    c.line([(43, 50), (53, 26)], width=2)
    c.ellipse(55, 64, 61, 70, filled=True, width=2)
    threshold(c)


@icon
def door_sliding(c):
    c.rect(12, 14, 84, 78)
    c.line([(48, 14), (48, 78)], width=4)
    c.line([(26, 46), (40, 46)], width=3)
    c.line([(32, 40), (26, 46), (32, 52)], width=3)
    c.line([(70, 46), (56, 46)], width=3)
    c.line([(64, 40), (70, 46), (64, 52)], width=3)
    threshold(c)


# --- windows -------------------------------------------------------------------------------


@icon
def window_standard(c):
    window(c)


@icon
def window_round(c):
    c.ellipse(14, 14, 82, 82)
    c.line([(48, 14), (48, 82)], width=4)
    c.line([(14, 48), (82, 48)], width=4)


@icon
def window_wide(c):
    c.rect(8, 28, 88, 68)
    c.line([(35, 28), (35, 68)], width=4)
    c.line([(61, 28), (61, 68)], width=4)


@icon
def window_tall(c):
    c.rect(32, 10, 64, 86)
    c.line([(32, 34), (64, 34)], width=4)
    c.line([(32, 58), (64, 58)], width=4)


@icon
def window_bay(c):
    c.line([(12, 66), (28, 30), (68, 30), (84, 66)])
    c.line([(48, 30), (48, 66)], width=4)
    c.line([(12, 66), (84, 66)])


# --- furniture -----------------------------------------------------------------------------


@icon
def furniture_sofa(c):
    furniture(c)


@icon
def furniture_table(c):
    c.rect(12, 30, 84, 42, radius=3)
    c.line([(22, 42), (22, 78)])
    c.line([(74, 42), (74, 78)])


@icon
def furniture_bed(c):
    c.rect(10, 34, 86, 66)
    c.rect(16, 42, 40, 56, width=4, radius=3)
    c.line([(10, 24), (10, 74)])
    c.line([(86, 50), (86, 74)])


@icon
def furniture_chair(c):
    c.line([(32, 14), (32, 52), (66, 52)])
    c.line([(32, 14), (28, 14)])
    c.line([(32, 52), (30, 82)])
    c.line([(66, 52), (68, 82)])


@icon
def furniture_wardrobe(c):
    c.rect(18, 10, 78, 84)
    c.line([(48, 10), (48, 84)], width=4)
    c.ellipse(40, 44, 44, 48, filled=True, width=2)
    c.ellipse(52, 44, 56, 48, filled=True, width=2)


if __name__ == "__main__":
    for name, function in ICONS.items():
        canvas = Canvas()
        function(canvas)
        canvas.save(name)
    print(f"wrote {len(ICONS)} icons")
