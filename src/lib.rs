use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::keyboard::Scancode;

use std::time::Instant;
use std::convert::TryInto;
use std::io::prelude::*;
use std::process::{ Command, Stdio };
use std::ffi::c_void;
use std::path::Path;
use std::fs::File;
use std::io::BufWriter;
use std::time::{ SystemTime, UNIX_EPOCH };

pub mod shader;
use crate::shader::*;

// General config rendering
#[derive(Debug)]
pub struct FragConf{
    cw: i32,
    ch: i32,
    ww: i32,
    wh: i32,
    pixelate: bool,
    streamer: Option<ShaderStreamer>,
}
// Config for rendering to file
#[derive(Debug)]
pub struct FFmpegConf{
    base: FragConf,
    framerate: u32,
    preset: String,
    tune: String,
    crf: u32,
    length: usize,
    output: String,
}

#[derive(Debug)]
pub enum Preset{
    UltraFast, SuperFast, VeryFast, Faster, Fast, Medium, Slow, Slower, VerySlow
}

#[derive(Debug)]
pub enum Tune{
    Film, Animation, Grain, StillImage, FastDecode, ZeroLatency
}

impl FragConf{
    pub fn new() -> Self{
        Self{
            cw: 0,
            ch: 0,
            ww: 0,
            wh: 0,
            pixelate: false,
            streamer: None,
        }
    }

    pub fn with_window_width(mut self, ww: u32) -> Self{
        let ww = ww as i32;
        if self.cw == 0 { self.cw = ww }
        self.ww = ww;
        self
    }

    pub fn with_window_height(mut self, wh: u32) -> Self{
        let wh = wh as i32;
        if self.ch == 0 { self.ch = wh }
        self.wh = wh;
        self
    }

    pub fn with_canvas_width(mut self, cw: u32) -> Self{
        let cw = cw as i32;
        if self.ww == 0 { self.ww = cw }
        self.cw = cw;
        self
    }

    pub fn with_canvas_height(mut self, ch: u32) -> Self{
        let ch = ch as i32;
        if self.wh == 0 { self.wh = ch }
        self.ch = ch;
        self
    }

    pub fn with_pixelate(mut self, pixelate: bool) -> Self{
        self.pixelate = pixelate;
        self
    }

    pub fn with_streamer(mut self, streamer: ShaderStreamer) -> Self{
        self.streamer = Some(streamer);
        self
    }

    pub fn into_ffmpeg_renderer(self) -> FFmpegConf{
        FFmpegConf{
            base: self,
            framerate: 30,
            crf: 20,
            preset: String::from("medium"),
            tune: String::from("film"),
            length: 60,
            output: String::from("output.mp4"),
        }
    }

    pub fn run_live(self) -> Result<(), String>{
        let streamer = if let Some(streamer) = self.streamer { streamer }
        else {
            println!("Frag: no streamer found, will use test streamer.");
            ShaderStreamer::test()
        };
        run(self.cw, self.ch, self.ww, self.wh, self.pixelate, streamer)
    }
}

impl Default for FragConf {
    fn default() -> Self {
        Self::new()
    }
}

impl FFmpegConf{
    pub fn with_framerate(mut self, fr: u32) -> Self{
        self.framerate = fr;
        self
    }

    pub fn with_crf(mut self, crf: u32) -> Self{
        self.crf = crf.min(51);
        self
    }

    pub fn with_preset(mut self, preset: Preset) -> Self{
        self.preset = match preset{
            Preset::UltraFast => "ultrafast",
            Preset::SuperFast => "superfast",
            Preset::VeryFast => "veryfast",
            Preset::Faster => "faster",
            Preset::Fast => "fast",
            Preset::Medium => "medium",
            Preset::Slow => "slow",
            Preset::Slower => "slower",
            Preset::VerySlow => "veryslow",
        }.to_string();
        self
    }

    pub fn with_tune(mut self, tune: Tune) -> Self{
        self.tune = match tune{
            Tune::Film => "film",
            Tune::Animation => "animation",
            Tune::Grain => "grain",
            Tune::StillImage => "stillimage",
            Tune::FastDecode => "fastdecode",
            Tune::ZeroLatency => "zerolatency",
        }.to_string();
        self
    }

    pub fn with_length(mut self, frames: usize) -> Self{
        self.length = frames;
        self
    }

    pub fn with_output(mut self, filename: &str) -> Self{
        self.output = filename.to_string();
        self
    }

    pub fn render(mut self) -> Result<(), String>{
        let streamer = if let Some(streamer) = self.base.streamer {
            self.base.streamer = None;
            streamer
        }
        else {
            return Err("Frag: no streamer found.".to_string());
        };
        render(self, streamer)
    }
}

trait StringErr<U, V>{
    fn strerr(self, msg: &str) -> Result<U, String>;
    fn strerr_prop(self, f: &dyn Fn(V) -> String) -> Result<U, String>;
}

impl<U, V> StringErr<U, V> for Result<U, V>{
    fn strerr(self, msg: &str) -> Result<U, String>{
        match self{
            Err(_) => Err(msg.to_string()),
            Ok(x) => Ok(x),
        }
    }

    fn strerr_prop(self, f: &dyn Fn(V) -> String) -> Result<U, String>{
        match self{
            Err(e) => Err(f(e)),
            Ok(x) => Ok(x),
        }
    }
}

fn render(conf: FFmpegConf, mut streamer: ShaderStreamer) -> Result<(), String> {
    let (sdl_context, _window, _gl_contex, _) = init_context(conf.base.ww, conf.base.wh).strerr("Frag: could not create context.")?;

    let (mut render_program, post_program) = init_programs(&mut streamer);
    let (i_time, i_delta_time, i_frame, _, _) = init_uniforms(&mut render_program, conf.base.cw, conf.base.ch);
    let vao = init_quad();
    let (canvas_fbo, canvas_tex) = init_rendertarget(conf.base.cw, conf.base.ch, conf.base.pixelate)?;

    let (mut t, mut dt, mut frame, mut sec) = (0.0, 0.0, 0usize, 0.0);
    let mut event_pump = sdl_context.event_pump().unwrap();

    // FFmpeg code adapted from:
    // http://blog.mmacklin.com/2013/06/11/real-time-video-capture-with-ffmpeg/
    let command = "ffmpeg";
    let args = ["-r", &format!("{}", conf.framerate), "-f", "rawvideo", "-pix_fmt", "rgba", "-s", &format!("{}x{}", conf.base.ww, conf.base.wh),
        "-i", "-", "-threads", "0", "-preset", &conf.preset, "-tune", &conf.tune,
        "-y", "-pix_fmt", "yuv420p", "-crf", &format!("{}", conf.crf),
        "-vf", "vflip", &conf.output];
    let process = Command::new(command)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .strerr_prop(&|e| format!("Frag: couldn't spawn ffmpeg: {}", e))?;

    let mut stdin = process.stdin.unwrap();

    let start = Instant::now();
    'running: loop {
        let lt = t;
        for event in event_pump.poll_iter() {
            if let Event::Quit{ .. } = event{
                break 'running;
            }
        }
        unsafe{
            // render to texture
            gl::BindFramebuffer(gl::FRAMEBUFFER, canvas_fbo);
            render_program.set_used();
            gl::Viewport(0, 0, conf.base.cw, conf.base.ch);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::BindVertexArray(vao);
            i_time.set_1f(t);
            i_delta_time.set_1f(dt);
            i_frame.set_1ui(frame.try_into().unwrap());
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
            //render to screen, skip if there is no scaling
            if !(conf.base.ww == conf.base.cw && conf.base.wh == conf.base.ch){
                gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
                post_program.set_used();
                gl::Viewport(0, 0, conf.base.ww, conf.base.wh);
                gl::BindTexture(gl::TEXTURE_2D, canvas_tex);
                gl::DrawArrays(gl::TRIANGLES, 0, 6);
            }
        }

        let mut buffer: Vec<u8> = vec![0; (conf.base.ww * conf.base.wh) as usize * 4];

        unsafe{
            gl::ReadPixels(0, 0, conf.base.ww, conf.base.wh, gl::RGBA, gl::UNSIGNED_BYTE, buffer.as_mut_ptr() as *mut c_void);
        }

        stdin.write_all(&buffer).strerr_prop(&|why| format!("Frag: couldn't write to ffmpeg stdin: {}", why))?;

        frame += 1;
        if frame > conf.length { break; }
        t += 1.0 / conf.framerate as f32;
        let rt = start.elapsed().as_millis() as f32 / 1000.0;
        dt = t - lt;
        if rt.floor() > sec{
            sec = rt.floor();
            println!("{} / {} frames", frame, conf.length);
            std::io::stdout().flush().strerr("Frag: could not flush stdout.")?;
        }
    }

    unsafe{
        gl::DeleteFramebuffers(1, &canvas_fbo);
    }

    std::mem::drop(stdin);

    let mut s = String::new();
    match process.stdout.unwrap().read_to_string(&mut s) {
        Err(why) => println!("couldn't read ffmpeg stdout: {}", why),
        Ok(_) => println!("ffmpeg responded with:\n{}", s),
    }
    Ok(())
}

fn run(cw: i32, ch: i32, ww: i32, wh: i32, pixelate: bool, mut streamer: ShaderStreamer) -> Result<(), String>{
    let (sdl_context, window, _gl_contex, _) = init_context(ww, wh).strerr("Frag: could not create context.")?;

    let (mut render_program, post_program) = init_programs(&mut streamer);
    let (mut i_time, mut i_delta_time, mut i_frame, mut i_aspect, mut i_resolution) = init_uniforms(&mut render_program, cw, ch);
    let vao = init_quad();
    let (canvas_fbo, canvas_tex) = init_rendertarget(cw, ch, pixelate)?;
    // initialize render target

    let (mut t, mut dt, mut frame, mut sec, mut last_frames, mut play_t) = (0.0, 0.0, 0, 0.0, 0, 0.0);
    let mut event_pump = sdl_context.event_pump().unwrap();
    streamer.start();
    let start = Instant::now();
    let mut playing = true;
    let mut lt;
    'running: loop {
        lt = t;
        let mut need_refresh = false;
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. }
                    => { break 'running  },
                Event::KeyDown { keycode: Some(Keycode::Space), .. }
                    => {
                        playing = !playing;
                        lt = start.elapsed().as_millis() as f32 / 1000.0;
                    }
                Event::KeyDown { keycode: Some(Keycode::Return), .. }
                    => {
                        let filename = match SystemTime::now().duration_since(UNIX_EPOCH){
                            Ok(n) => format!("{}.png", n.as_secs()),
                            Err(_) => "0.png".to_string(),
                        };
                        let path = Path::new(&filename);
                        let file = File::create(path).expect("Frag: could not open file for frame image.");
                        let w = &mut BufWriter::new(file);

                        let mut encoder = png::Encoder::new(w, ww as u32, wh as u32);
                        encoder.set_color(png::ColorType::RGBA);
                        encoder.set_depth(png::BitDepth::Eight);
                        let mut writer = encoder.write_header().expect("Frag: could not write png header for frame image.");

                        let mut buffer: Vec<u8> = vec![0; (ww * wh) as usize * 4];
                        unsafe{
                            gl::ReadPixels(0, 0, ww, wh, gl::RGBA, gl::UNSIGNED_BYTE, buffer.as_mut_ptr() as *mut c_void);
                        }

                        if let Err(e) = writer.write_image_data(&buffer){
                            println!("Frag: could not save frame image: {}", e);
                        }
                    },
                Event::KeyDown { keycode: Some(Keycode::Down), .. }
                    => {
                        play_t = 0.0;
                        lt = start.elapsed().as_millis() as f32 / 1000.0;
                        need_refresh = true;
                    },
                Event::KeyDown { keycode: Some(Keycode::PageUp), .. }
                    => {
                        play_t += 5.0;
                        lt = start.elapsed().as_millis() as f32 / 1000.0;
                        need_refresh = true;
                    },
                Event::KeyDown { keycode: Some(Keycode::PageDown), .. }
                    => {
                        play_t -= 5.0;
                        lt = start.elapsed().as_millis() as f32 / 1000.0;
                        need_refresh = true;
                    },
                _ => {}
            }
        }
        need_refresh = need_refresh || if event_pump.keyboard_state().is_scancode_pressed(Scancode::Left){
            play_t -= if playing { 1.0 / 15.0 } else { 1.0 / 30.0 };
            lt = start.elapsed().as_millis() as f32 / 1000.0;
            true
        } else if event_pump.keyboard_state().is_scancode_pressed(Scancode::Right){
            play_t += 1.0 / 30.0;
            lt = start.elapsed().as_millis() as f32 / 1000.0;
            true
        }else {
            false
        };
        // rebuild shader if needed
        need_refresh = need_refresh || if streamer.is_dirty(){
            println!("Frag: rebuilding shader.");
            match streamer.build(false){
                Ok(program) => {
                    render_program = program;
                    render_program.set_used();
                    i_aspect.reload(&render_program);
                    i_resolution.reload(&render_program);
                    i_time.reload(&render_program);
                    i_delta_time.reload(&render_program);
                    i_frame.reload(&render_program);
                    i_aspect.set_1f(cw as f32 / ch as f32);
                    i_resolution.set_2f(cw as f32, ch as f32);
                    true
                },
                Err(e) => {
                    println!("Frag: could not rebuild shader: {}", e);
                    false
                },
            }
        } else {
            false
        };
        // render
        if need_refresh || playing{
            unsafe{
                // render to texture
                gl::BindFramebuffer(gl::FRAMEBUFFER, canvas_fbo);
                render_program.set_used();
                gl::Viewport(0, 0, cw as i32, ch as i32);
                gl::Clear(gl::COLOR_BUFFER_BIT);
                gl::BindVertexArray(vao);
                i_time.set_1f(play_t);
                i_delta_time.set_1f(dt);
                i_frame.set_1ui(frame);
                gl::DrawArrays(gl::TRIANGLES, 0, 6);
                //render to screen
                gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
                post_program.set_used();
                gl::Viewport(0, 0, ww, wh);
                gl::BindTexture(gl::TEXTURE_2D, canvas_tex);
                gl::DrawArrays(gl::TRIANGLES, 0, 6);
            }
            window.gl_swap_window();
            frame += 1;
            if playing{
                t = start.elapsed().as_millis() as f32 / 1000.0;
                dt = t - lt;
                play_t += dt;
            }
            if t.floor() > sec{
                print!("{}, ", frame - last_frames);
                last_frames = frame;
                sec = t.floor();
                std::io::stdout().flush().strerr("Frag: could not flush stdout.")?;
            }
        }
    }

    unsafe{
        gl::DeleteFramebuffers(1, &canvas_fbo);
    }

    Ok(())
}

fn init_context(ww: i32, wh: i32) -> Result<(sdl2::Sdl,sdl2::video::Window,sdl2::video::GLContext,()), String>{
    let sdl_context = sdl2::init()?;//.expect("Frag: could not create SDL context.");
    let video_subsystem = sdl_context.video()?;//.expect("Frag: could not get SDL video subsystem.");

    // window dimension must be the same or bigger as render dimensions, :/
    let window = video_subsystem.window(":3", ww as u32, wh as u32)
        .position_centered().opengl().build().strerr("Frag: could not create window.")?;

    let _gl_contex = window.gl_create_context().strerr("Frag: could not create GL context.")?; //needs to exist
    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(4, 5);

    #[allow(dead_code)]
    let _gl = gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const std::os::raw::c_void);

    Ok((sdl_context, window, _gl_contex, _gl))
}

fn init_programs(streamer: &mut ShaderStreamer) -> (Program, Program){
    let render_program = match streamer.build(true){
        Ok(program) => program,
        Err(e) => {
            println!("Frag: could not build program: {}", e);
            Program::new(RENDER_VERT_SRC, &format!("{}{}", RENDER_FRAG_HEADER, RENDER_FRAG_STD_BODY))
                .expect("Frag: could not create standard program.")
        },
    };
    let post_program = Program::new(POST_VERT_SRC, POST_FRAG_SRC).expect("Frag: could not create post program.");
    (render_program, post_program)
}

fn init_uniforms(render_program: &mut Program, cw: i32, ch: i32) -> (Uniform, Uniform, Uniform, Uniform, Uniform){
    render_program.set_used();
    let i_time = Uniform::new(&render_program, "iTime").with_1f(0.0);
    let i_delta_time = Uniform::new(&render_program, "iDeltaTime").with_1f(0.0);
    let i_frame = Uniform::new(&render_program, "iFrame").with_1ui(0);
    let i_aspect = Uniform::new(&render_program, "iAspect").with_1f(cw as f32 / ch as f32);
    let i_resolution = Uniform::new(&render_program, "iResolution").with_2f(cw as f32, ch as f32);
    (i_time, i_delta_time, i_frame, i_aspect, i_resolution)
}

fn init_quad() -> gl::types::GLuint{
    let vertices: Vec<f32> = vec![-1., -1., 0., -1., 1., 0., 1., 1., 0., -1., -1., 0., 1., 1., 0., 1., -1., 0.];
    let mut vbo: gl::types::GLuint = 0;
    let mut vao: gl::types::GLuint = 0;

    unsafe {
        gl::GenBuffers(1, &mut vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER, // target
            (vertices.len() * std::mem::size_of::<f32>()) as gl::types::GLsizeiptr, // size of data in bytes
            vertices.as_ptr() as *const gl::types::GLvoid, // pointer to data
            gl::STATIC_DRAW // usage
        );
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::EnableVertexAttribArray(0);
        gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, (3 * std::mem::size_of::<f32>()) as gl::types::GLint, std::ptr::null());
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::BindVertexArray(0);
    }
    vao
}

fn init_rendertarget(cw: i32, ch: i32, pixelate: bool) -> Result<(gl::types::GLuint, gl::types::GLuint), String>{
    let mut canvas_fbo: gl::types::GLuint = 0;
    let mut canvas_tex: gl::types::GLuint = 0;

    unsafe{
        gl::GenFramebuffers(1, &mut canvas_fbo);
        gl::BindFramebuffer(gl::FRAMEBUFFER, canvas_fbo);
        gl::GenTextures(1, &mut canvas_tex);
        gl::BindTexture(gl::TEXTURE_2D, canvas_tex);

        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGB as i32, cw, ch, 0, gl::RGB, gl::UNSIGNED_BYTE, std::ptr::null());
        let filter = if pixelate { gl::NEAREST } else { gl::LINEAR } as i32;
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, filter);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, filter);

        gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, canvas_tex, 0);
        if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE{
            return Err("Frag: could not initialize canvas framebuffer.".to_string());
        }
    }

    Ok((canvas_fbo, canvas_tex))
}

// OpenGl code stolen from these sources
// https://nercury.github.io/rust/opengl/tutorial/2018/02/10/opengl-in-rust-from-scratch-03-compiling-shaders.html
// https://nercury.github.io/rust/opengl/tutorial/2018/02/11/opengl-in-rust-from-scratch-04-triangle.html
// https://learnopengl.com/Advanced-OpenGL/Framebuffers

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
