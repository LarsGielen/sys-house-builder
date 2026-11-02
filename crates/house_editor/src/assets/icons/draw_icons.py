#!/usr/bin/env python3
"""Draws the house editor icons at any size.

Icons are authored on a 24 unit grid; every size is rendered directly from the
shape definitions (not scaled from a bitmap), so they stay crisp.

Examples:
  ./draw_icons.py --size 128 --out out            # all icons, 128x128
  ./draw_icons.py --size 512 wall door stairs     # only some icons
  ./draw_icons.py --list
  ./draw_icons.py --size 64 --color 30,30,30      # dark icons instead of white
  ./draw_icons.py --sheet --size 64               # one contact sheet with labels
"""
import argparse
import math
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

GRID = 24.0
STROKE_WIDTH = 1.8


class Canvas:
    def __init__(self, size, mirrored=False):
        self.size = size
        self.pixels_per_unit = size / GRID
        self.coverage = np.zeros((size, size), dtype=np.float32)
        centers = (np.arange(size, dtype=np.float32) + 0.5) / self.pixels_per_unit
        self.x, self.y = np.meshgrid(centers, centers)
        if mirrored:
            self.x = GRID - self.x

    def paint(self, signed_distance):
        alpha = np.clip(0.5 - signed_distance * self.pixels_per_unit, 0.0, 1.0)
        np.maximum(self.coverage, alpha, out=self.coverage)

    def stroke(self, distance_to_centerline):
        self.paint(distance_to_centerline - STROKE_WIDTH / 2)

    def line(self, start, end):
        (x0, y0), (x1, y1) = start, end
        dx, dy = x1 - x0, y1 - y0
        along = np.clip(((self.x - x0) * dx + (self.y - y0) * dy) / (dx * dx + dy * dy), 0, 1)
        self.stroke(np.hypot(self.x - (x0 + dx * along), self.y - (y0 + dy * along)))

    def polyline(self, points):
        for start, end in zip(points, points[1:]):
            self.line(start, end)

    def polygon(self, points):
        self.polyline(points)
        self.line(points[-1], points[0])

    def circle(self, center, radius):
        self.stroke(np.abs(np.hypot(self.x - center[0], self.y - center[1]) - radius))

    def disc(self, center, radius):
        self.paint(np.hypot(self.x - center[0], self.y - center[1]) - radius)

    def arc(self, center, radius, from_degrees, to_degrees):
        """Degrees are clockwise from the positive x axis (y points down)."""
        cx, cy = center
        span = to_degrees - from_degrees
        endpoints = [
            (cx + math.cos(math.radians(a)) * radius, cy + math.sin(math.radians(a)) * radius)
            for a in (from_degrees, to_degrees)
        ]
        angle = np.degrees(np.arctan2(self.y - cy, self.x - cx))
        on_arc = np.mod(angle - from_degrees, 360.0) <= span
        ring = np.abs(np.hypot(self.x - cx, self.y - cy) - radius)
        caps = np.minimum(*(np.hypot(self.x - ex, self.y - ey) for ex, ey in endpoints))
        self.stroke(np.where(on_arc, ring, caps))

    def rect(self, low, high, corner_radius):
        center = ((low[0] + high[0]) / 2, (low[1] + high[1]) / 2)
        half = ((high[0] - low[0]) / 2, (high[1] - low[1]) / 2)
        inner_x = np.abs(self.x - center[0]) - (half[0] - corner_radius)
        inner_y = np.abs(self.y - center[1]) - (half[1] - corner_radius)
        outside = np.hypot(np.maximum(inner_x, 0), np.maximum(inner_y, 0))
        inside = np.minimum(np.maximum(inner_x, inner_y), 0)
        self.stroke(np.abs(outside + inside - corner_radius))


def draw(icon, c):
    if icon == "select":
        c.polygon([(6, 3), (6, 19), (10, 15), (13, 21), (15.5, 19.8), (12.5, 14), (18, 14)])
    elif icon == "wall":
        c.rect((3, 6), (21, 18), 1)
        c.line((3, 12), (21, 12))
        c.line((9, 6), (9, 12))
        c.line((15, 12), (15, 18))
    elif icon == "arc_wall":
        c.arc((12, 19), 12, 215, 325)
        c.disc((3.1, 12.1), 2)
        c.disc((20.9, 12.1), 2)
    elif icon == "room":
        c.rect((4, 4), (20, 20), 1)
        c.line((12, 4), (12, 13))
        c.line((12, 13), (20, 13))
    elif icon == "door":
        c.rect((6, 3), (18, 21), 1)
        c.disc((14.5, 12.5), 1.1)
    elif icon == "window":
        c.rect((4, 5), (20, 19), 1)
        c.line((12, 5), (12, 19))
        c.line((4, 12), (20, 12))
    elif icon == "door_glass":
        c.rect((6, 3), (18, 21), 1)
        c.rect((9, 6), (15, 13), 0.5)
        c.disc((14.5, 17), 1)
    elif icon == "door_arched":
        c.line((6, 21), (6, 10))
        c.arc((12, 10), 6, 180, 360)
        c.line((18, 10), (18, 21))
        c.line((4, 21), (20, 21))
        c.disc((15, 15.5), 1)
    elif icon == "door_double":
        c.rect((3, 3), (21, 21), 1)
        c.line((12, 3), (12, 21))
        c.disc((9.5, 12.5), 1)
        c.disc((14.5, 12.5), 1)
    elif icon == "window_single":
        c.rect((5, 4), (19, 20), 1)
    elif icon == "window_arched":
        c.line((5, 20), (5, 10))
        c.arc((12, 10), 7, 180, 360)
        c.line((19, 10), (19, 20))
        c.line((5, 20), (19, 20))
        c.line((12, 3), (12, 20))
    elif icon == "window_round":
        c.circle((12, 12), 8.5)
        c.line((12, 3.5), (12, 20.5))
        c.line((3.5, 12), (20.5, 12))
    elif icon == "stairs":
        c.polyline([(3, 20), (9, 20), (9, 15), (14, 15), (14, 10), (19, 10), (19, 5), (22, 5)])
    elif icon == "furniture":
        c.rect((6, 5), (18, 12), 2)
        c.rect((3, 11), (21, 18), 2)
        c.line((6, 18), (6, 20.5))
        c.line((18, 18), (18, 20.5))
    elif icon == "paint":
        c.rect((4, 4), (17, 9), 1.5)
        c.polyline([(17, 6.5), (20, 6.5), (20, 12), (12, 12), (12, 14)])
        c.rect((10, 14), (14, 21), 1)
    elif icon == "roof":
        c.polyline([(2.5, 12), (12, 4), (21.5, 12)])
        c.polyline([(5.5, 10), (5.5, 20), (18.5, 20), (18.5, 10)])
        c.rect((10, 14), (14, 20), 0.5)
    elif icon in ("undo", "redo"):
        c.polyline([(8, 5.5), (4, 9.5), (8, 13.5)])
        c.line((4, 9.5), (14, 9.5))
        c.arc((14, 14.5), 5, -90, 90)
        c.line((14, 19.5), (9, 19.5))
    elif icon == "save":
        c.rect((4, 4), (20, 20), 2)
        c.rect((8, 4), (15, 9), 0.5)
        c.rect((7, 13), (17, 20), 0.5)
    elif icon == "open":
        c.polyline([(3, 19), (3, 5.5), (9, 5.5), (11, 8), (18, 8), (18, 11)])
        c.polygon([(3, 19), (6, 11), (22, 11), (19, 19)])
    elif icon == "trash":
        c.line((4, 7), (20, 7))
        c.polyline([(9, 7), (9, 4), (15, 4), (15, 7)])
        c.polyline([(6, 7), (7, 20), (17, 20), (18, 7)])
        c.line((10, 11), (10, 16))
        c.line((14, 11), (14, 16))
    elif icon == "duplicate":
        c.rect((4, 4), (15, 15), 2)
        c.rect((9, 9), (20, 20), 2)
    elif icon in ("rotate", "rotate_counter_clockwise"):
        c.arc((12, 12), 7.5, 10, 300)
        c.polyline([(17.5, 2.5), (17.5, 7), (13, 7)])
    elif icon == "move":
        c.line((12, 3), (12, 21))
        c.line((3, 12), (21, 12))
        c.polyline([(9, 6), (12, 3), (15, 6)])
        c.polyline([(9, 18), (12, 21), (15, 18)])
        c.polyline([(6, 9), (3, 12), (6, 15)])
        c.polyline([(18, 9), (21, 12), (18, 15)])
    elif icon == "layers":
        c.polygon([(12, 3), (21, 8), (12, 13), (3, 8)])
        c.polyline([(3, 12), (12, 17), (21, 12)])
        c.polyline([(3, 16), (12, 21), (21, 16)])
    elif icon == "plus":
        c.line((12, 5), (12, 19))
        c.line((5, 12), (19, 12))
    elif icon == "minus":
        c.line((5, 12), (19, 12))
    elif icon == "check":
        c.polyline([(5, 12.5), (10, 17.5), (19, 7)])
    elif icon == "close":
        c.line((6, 6), (18, 18))
        c.line((18, 6), (6, 18))
    elif icon == "chevron_up":
        c.polyline([(6, 15), (12, 9), (18, 15)])
    elif icon == "chevron_down":
        c.polyline([(6, 9), (12, 15), (18, 9)])
    elif icon == "chevron_left":
        c.polyline([(15, 6), (9, 12), (15, 18)])
    elif icon == "chevron_right":
        c.polyline([(9, 6), (15, 12), (9, 18)])
    elif icon == "grid":
        c.rect((4, 4), (20, 20), 1.5)
        c.line((4, 9.3), (20, 9.3))
        c.line((4, 14.7), (20, 14.7))
        c.line((9.3, 4), (9.3, 20))
        c.line((14.7, 4), (14.7, 20))
    elif icon == "magnet":
        c.arc((12, 10), 7, 180, 360)
        c.arc((12, 10), 3, 180, 360)
        c.line((5, 10), (5, 20))
        c.line((9, 10), (9, 20))
        c.line((15, 10), (15, 20))
        c.line((19, 10), (19, 20))
        c.line((5, 20), (9, 20))
        c.line((15, 20), (19, 20))
    elif icon in ("zoom_in", "zoom_out"):
        c.circle((10.5, 10.5), 6.5)
        c.line((15.5, 15.5), (21, 21))
        c.line((7.5, 10.5), (13.5, 10.5))
        if icon == "zoom_in":
            c.line((10.5, 7.5), (10.5, 13.5))
    elif icon == "frame":
        c.polyline([(4, 9), (4, 4), (9, 4)])
        c.polyline([(15, 4), (20, 4), (20, 9)])
        c.polyline([(20, 15), (20, 20), (15, 20)])
        c.polyline([(9, 20), (4, 20), (4, 15)])
        c.disc((12, 12), 2.5)
    elif icon == "person":
        c.circle((12, 7.5), 3.5)
        c.arc((12, 20), 7, 180, 360)
        c.line((5, 20), (19, 20))
    elif icon == "play":
        c.polygon([(7, 5), (19, 12), (7, 19)])
    elif icon == "sliders":
        c.line((4, 7), (20, 7))
        c.line((4, 12), (20, 12))
        c.line((4, 17), (20, 17))
        c.disc((9, 7), 2.6)
        c.disc((15, 12), 2.6)
        c.disc((8, 17), 2.6)
    elif icon == "info":
        c.circle((12, 12), 9)
        c.line((12, 11), (12, 17))
        c.disc((12, 7.7), 1.1)
    elif icon == "exit":
        c.polyline([(9, 4), (4, 4), (4, 20), (9, 20)])
        c.line((9, 12), (20, 12))
        c.polyline([(16, 8), (20, 12), (16, 16)])
    elif icon == "split":
        c.line((3, 12), (9, 12))
        c.line((15, 12), (21, 12))
        c.line((12, 5), (12, 19))
    elif icon == "merge":
        c.line((3, 12), (10, 12))
        c.line((14, 12), (21, 12))
        c.polyline([(7, 8), (11, 12), (7, 16)])
        c.polyline([(17, 8), (13, 12), (17, 16)])
    elif icon == "reshape":
        c.polyline([(5, 18), (12, 6), (19, 18)])
        c.disc((5, 18), 2.4)
        c.disc((12, 6), 2.4)
        c.disc((19, 18), 2.4)
    elif icon == "bed":
        c.rect((3, 11), (21, 18), 1.5)
        c.rect((5, 7), (10, 11), 1)
        c.line((3, 18), (3, 20.5))
        c.line((21, 18), (21, 20.5))
    elif icon == "table":
        c.rect((3, 8), (21, 11), 1)
        c.line((6, 11), (6, 20))
        c.line((18, 11), (18, 20))
    elif icon == "toilet":
        c.rect((8, 3), (16, 8), 1.5)
        c.line((6, 11), (18, 11))
        c.arc((12, 11), 6, 0, 180)
        c.line((12, 17), (12, 20))
        c.line((9, 20), (15, 20))
    elif icon == "stove":
        c.rect((4, 3), (20, 21), 2)
        c.line((4, 8), (20, 8))
        c.circle((9, 14.5), 2.5)
        c.circle((15, 14.5), 2.5)
    elif icon == "wardrobe":
        c.rect((5, 3), (19, 21), 1.5)
        c.line((12, 3), (12, 21))
        c.disc((10, 12), 0.9)
        c.disc((14, 12), 0.9)
    elif icon == "tag":
        c.polygon([(3, 4), (12, 4), (21, 13), (13, 21), (3, 11)])
        c.disc((8, 8.5), 1.4)
    elif icon == "angle":
        c.line((4, 19), (21, 19))
        c.line((4, 19), (17, 5))
        c.arc((4, 19), 9, -47, 0)
    elif icon == "ruler":
        c.rect((2, 8), (22, 16), 1)
        c.line((6, 8), (6, 12))
        c.line((10, 8), (10, 11))
        c.line((14, 8), (14, 12))
        c.line((18, 8), (18, 11))
    elif icon == "edit":
        c.polygon([(4, 20), (5, 15), (16, 4), (20, 8), (9, 19)])
        c.line((13.5, 6.5), (17.5, 10.5))
    else:
        raise KeyError(icon)


ICONS = """select wall arc_wall room door door_glass door_arched door_double window window_single
window_arched window_round stairs furniture paint roof undo redo save open trash duplicate rotate
rotate_counter_clockwise move layers plus minus check close chevron_up chevron_down chevron_left
chevron_right grid magnet zoom_in zoom_out frame person play sliders info exit split merge reshape
bed table toilet stove wardrobe tag edit angle ruler""".split()
MIRRORED = {"redo", "rotate_counter_clockwise"}


def render(icon, size, color=(255, 255, 255)):
    canvas = Canvas(size, mirrored=icon in MIRRORED)
    draw(icon, canvas)
    alpha = np.round(canvas.coverage * 255).astype(np.uint8)
    pixels = np.zeros((size, size, 4), dtype=np.uint8)
    pixels[..., :3] = color
    pixels[..., 3] = alpha
    return Image.fromarray(pixels, "RGBA")


def contact_sheet(names, size, color):
    columns = 8
    label_height = 16
    padding = 12
    rows = math.ceil(len(names) / columns)
    cell_w, cell_h = size + padding * 2, size + label_height + padding * 2
    background = (235, 235, 235, 255) if sum(color) > 380 * 0 and max(color) > 200 else (255, 255, 255, 255)
    # white icons need a dark backdrop to be visible
    background = (40, 40, 48, 255) if min(color) > 200 else (245, 245, 245, 255)
    sheet = Image.new("RGBA", (cell_w * columns, cell_h * rows), background)
    pen = ImageDraw.Draw(sheet)
    for index, name in enumerate(names):
        left, top = (index % columns) * cell_w, (index // columns) * cell_h
        sheet.alpha_composite(render(name, size, color), (left + padding, top + padding))
        pen.text((left + padding, top + padding + size + 2), name, fill=(150, 150, 150, 255))
    return sheet


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("names", nargs="*", help="icons to draw (default: all)")
    parser.add_argument("--size", type=int, default=48, help="width and height in pixels (default 48)")
    parser.add_argument("--out", default="icons_out", help="output directory (default icons_out)")
    parser.add_argument("--color", default="255,255,255", help="R,G,B (default white)")
    parser.add_argument("--sheet", action="store_true", help="write one labelled contact sheet instead of files")
    parser.add_argument("--list", action="store_true", help="print icon names and exit")
    args = parser.parse_args()

    if args.list:
        print("\n".join(ICONS))
        return
    names = args.names or ICONS
    unknown = [name for name in names if name not in ICONS]
    if unknown:
        parser.error(f"unknown icons: {', '.join(unknown)} (see --list)")
    color = tuple(int(part) for part in args.color.split(","))

    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    if args.sheet:
        contact_sheet(names, args.size, color).save(out / "sheet.png")
        print(f"wrote {out / 'sheet.png'}")
        return
    for name in names:
        render(name, args.size, color).save(out / f"{name}.png")
    print(f"wrote {len(names)} icons at {args.size}px to {out}/")


if __name__ == "__main__":
    main()
