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
* Documentation
* Post process fragment shader accessable
