# Frag

Fragment shaders in Rust.
Let's you skip the boilerplate and just write a fragment shader.
Useful when for example, writing a raymarching shader.

## Features

* Compose shader of multiple part or files
* Live coding: updates when a file is updated
* Keyboard controls for time
* Resolution independent: separate resolutions for rendering and displaying. Can be used to stretch, pixelate or anti alias
* MP4 rendering with FFMPEG
* Save current frame as PNG image

## Controls

* Space: Pause/Resume time and rendering
* Left(hold): go back in time
* Right(hold): go forward in time
* Down: set time to 0
* PageDown: jump backward in time with 5 seconds
* PageUp: jump forward in time with 5 seconds

## Todo

* Post process fragment shader accessable

## Examples

### Example live coding, with pixel art like style

```rust
use frag::*;
let streamer = shader::ShaderStreamer::new()
    .with_file("lib.glsl")
    .with_file("shader.glsl");
FragConf::new()
    .with_window_width(1600)
    .with_window_height(900)
    .with_canvas_width(320)
    .with_canvas_height(180)
    .with_pixelate(true)
    .with_streamer(streamer)
    .run_live().expect("Could not run.");
```

### Example rendering to video

```rust
use frag::*;
let streamer = shader::ShaderStreamer::new()
    .with_file("lib.glsl")
    .with_file("shader.glsl");
FragConf::new()
    .with_window_width(1600)
    .with_window_height(900)
    .with_streamer(streamer)
    .into_ffmpeg_renderer()
    .with_framerate(30)
    .with_crf(20)
    .with_preset(Preset::Slow)
    .with_tune(Tune::Animation)
    .with_length(600)
    .with_output("render.mp4")
    .render().expect("Could not render.");
```

## License

```
Copyright (C) 2024 Cody Bloemhard

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
```
