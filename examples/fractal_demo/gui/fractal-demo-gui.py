# SPDX-FileCopyrightText: © 2024 Siemens AG
# SPDX-License-Identifier: MIT

from PIL import Image, ImageTk
import redis
from tkinter import Tk, Canvas, PhotoImage, mainloop
import sys

canvas_id = None
fractal_width = None
fractal_height = None
metadata_str = None
image_ref = None


def get_metadata():
    global metadata_str
    global fractal_width
    global fractal_height

    # example string : "1000,800,-1.2,0.35,-1.0,0.2"
    # => pixel chunks of 1000x800 pixels, containing pixel data from fractal section
    # top left: (-1.2, 0.35) to lower right: (-1.0, 0.2)
    new_metadata_str = rdb.get("fractal-metadata")
    updated = metadata_str != new_metadata_str
    if updated:

        metadata_str = new_metadata_str
        print(f"got updated metadata: '{metadata_str}")
        try:
            metadata = [float(token) for token in metadata_str.decode().split(",")]
        except ValueError as ex:
            print(
                f"failed to parse metadata string '{metadata_str}' with message '{ex}'"
            )
            return None

        if len(metadata) != 6:
            print(f"unexpected number of tokens in metadata string '{metadata_str}'")
            return None

        width, height, top_left_x, top_left_y, lower_right_x, lower_right_y = tuple(
            metadata
        )
        if fractal_width is not None and fractal_width != width:
            print(
                f"dynamic change of fractal width at runtime not supported (changed from {fractal_width} to {width}"
            )
            return None
        if fractal_height is not None and fractal_height != height:
            print(
                f"dynamic change of fractal height at runtime not supported (changed from {fractal_height} to {height}"
            )
            return None

        fractal_width = width
        fractal_height = height

    return updated, fractal_width, fractal_height


def get_pixel_data():
    pixel_data_str = rdb.get("fractal-chunk-0-data")
    if pixel_data_str:
        pixel_data = bytes.fromhex(pixel_data_str.decode())
        print(f"got {len(pixel_data)} bytes of pixel data")
    else:
        print(f"no data found in redis")
        return bytes([])
    return pixel_data


def update_image():
    global canvas_id
    global image_ref

    metadata = get_metadata()
    if metadata is None:
        print("getting metadata failed, quitting!")
        sys.exit(1)

    pixeldata = get_pixel_data()
    image_ref = ImageTk.PhotoImage(data=pixeldata)
    if canvas_id is not None:
        canvas.delete(canvas_id)
        canvas_id = None
    canvas_id = canvas.create_image(0, 0, image=image_ref, anchor="nw", state="normal")
    window.after(1000, update_image)


rdb = redis.Redis(host="localhost", port=6379, db=0)

metadata = get_metadata()
if metadata is None:
    print("getting metadata failed, quitting!")
    sys.exit(1)
_, width, height = metadata

window = Tk()
canvas = Canvas(window, width=width, height=height)
canvas.pack()

pixeldata = get_pixel_data()
image = ImageTk.PhotoImage(data=pixeldata)
canvas_id = canvas.create_image(0, 0, image=image, anchor="nw", state="normal")

window.after(1000, update_image)

mainloop()
