use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Instant;

pub mod shader;

use crate::shader::*;

pub fn run(cw: i32, ch: i32, ww: i32, wh: i32, pixelate: bool, mut builder: ShaderStreamer) {
    if cw <= 0 || ch <= 0 { panic!("cw or ch (canvas width or height) <= 0"); }
    if ww <= 0 || wh <= 0 { panic!("ww or wh (canvas width or height) <= 0"); }

    let sdl_context = sdl2::init().expect("Frag: could not create SDL context.");
    let video_subsystem = sdl_context.video().expect("Frag: could not get SDL video subsystem.");

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(4, 5);

    let window = video_subsystem.window(":3", ww as u32, wh as u32)
        .position_centered().opengl().build().expect("Frag: could not create window.");

    let _gl_contex = window.gl_create_context().expect("Frag: could not create GL context."); //needs to exist
    #[allow(dead_code)]
    let _gl = gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const std::os::raw::c_void);
    // build post vertex
    let mut render_program = match builder.build(true){
        Ok(program) => program,
        Err(e) => {
            println!("Frag: could not build program: {}", e);
            Program::new(RENDER_VERT_SRC, &format!("{}{}", RENDER_FRAG_HEADER, RENDER_FRAG_STD_BODY))
                .expect("Frag: could not create standard program.")
        },
    };
    let post_program = Program::new(POST_VERT_SRC, POST_FRAG_SRC).expect("Frag: could not create post program.");
    // set initial uniforms
    render_program.set_used();
    let mut i_time = Uniform::new(&render_program, "iTime").with_1f(0.0);
    let mut i_delta_time = Uniform::new(&render_program, "iDeltaTime").with_1f(0.0);
    let mut i_frame = Uniform::new(&render_program, "iFrame").with_1ui(0);
    let mut i_aspect = Uniform::new(&render_program, "iAspect").with_1f(cw as f32 / ch as f32);
    let mut i_resolution = Uniform::new(&render_program, "iResolution").with_2f(cw as f32, ch as f32);
    // initialize all geometry
    let vertices: Vec<f32> = vec![-1., -1., 0., -1., 1., 0., 1., 1., 0., -1., -1., 0., 1., 1., 0., 1., -1., 0.];
    let mut vbo: gl::types::GLuint = 0;
    let mut vao: gl::types::GLuint = 0;
    unsafe {
        gl::GenBuffers(1, &mut vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,                                                       // target
            (vertices.len() * std::mem::size_of::<f32>()) as gl::types::GLsizeiptr, // size of data in bytes
            vertices.as_ptr() as *const gl::types::GLvoid, // pointer to data
            gl::STATIC_DRAW,                               // usage
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
    // initialize render target
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
            panic!("Frag: could not initialize canvas framebuffer.");
        }
    }

    let mut t = 0.0;
    let mut dt = 0.0;
    let mut frame = 0;
    let mut event_pump = sdl_context.event_pump().unwrap();
    builder.start();
    let start = Instant::now();
    'running: loop {
        let lt = t;
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. }
                    => { break 'running  },
                _ => {}
            }
        }
        // rebuild shader if needed
        if builder.is_dirty(){
            println!("Frag: rebuilding shader.");
            match builder.build(false){
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
                },
                Err(e) => {
                    println!("Frag: could not rebuild shader: {}", e);
                },
            }
        }
        // render
        unsafe{
            // render to texture
            gl::BindFramebuffer(gl::FRAMEBUFFER, canvas_fbo);
            render_program.set_used();
            gl::Viewport(0, 0, cw as i32, ch as i32);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::BindVertexArray(vao);
            i_time.set_1f(t);
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
        t = start.elapsed().as_millis() as f32 / 1000.0;
        dt = t - lt;
        // print!("{}, ", dt);
    }

    unsafe{
        gl::DeleteFramebuffers(1, &canvas_fbo);
    }
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
