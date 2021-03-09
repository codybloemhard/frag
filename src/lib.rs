use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;
use std::time::Instant;
use std::ffi::{CStr, CString};

pub fn run(cw: i32, ch: i32, ww: u32, wh: u32) {
    if cw <= 0 || ch <= 0 { panic!("cw or ch (canvas width or height) <= 0"); }

    let sdl_context = sdl2::init().expect("Frag: could not create SDL context.");
    let video_subsystem = sdl_context.video().expect("Frag: could not get SDL video subsystem.");

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(4, 5);

    let window = video_subsystem.window(":3", ww, wh)
        .position_centered().opengl().build().expect("Frag: could not create window.");

    let _gl_contex = window.gl_create_context().expect("Frag: could not create GL context."); //needs to exist
    #[allow(dead_code)]
    let _gl = gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const std::os::raw::c_void);

    let render_vert_source =
    "
    #version 450 core
    layout (location = 0) in vec3 Position;
    uniform float iAspect;

    out vec2 uv;

    void main()
    {
        uv = Position.xy * 0.5;
        uv.x *= iAspect;
        gl_Position = vec4(Position, 1.0);
    }";

    let render_vert_shader = Shader::from_vert_source(&CString::new(render_vert_source).expect("Frag: could not create vertex c string."))
        .expect("Frag: could not create vertex shader.");

    let render_frag_source =
    "
    #version 450 core
    in vec2 uv;
    uniform float iTime;
    uniform float iAspect;
    uniform vec2 iResolution;

    out vec4 color;

    void main()
    {
        float rad = 0.4 + (sin(iTime) * 0.5 + 0.5) * 0.1;
        color = vec4(1.0f, 0.5f, 0.2f, 1.0f) * smoothstep(rad, rad-0.001, length(uv - vec2(0.0)));
    }";
    let render_frag_shader = Shader::from_frag_source(&CString::new(render_frag_source).expect("Frag: could not create fragment c string."))
        .expect("Frag: could not create fragment shader.");

    let render_program = Program::from_shaders(&[render_vert_shader, render_frag_shader]).expect("Frag: could not create shader program.");

    render_program.set_used();
    let i_time = Uniform::new(&render_program, "iTime").with_1f(0.0);
    let i_delta_time = Uniform::new(&render_program, "iDeltaTime").with_1f(0.0);
    let i_frame = Uniform::new(&render_program, "iFrame").with_1ui(0);
    let _i_aspect = Uniform::new(&render_program, "iAspect").with_1f(cw as f32 / ch as f32);
    let _i_resolution = Uniform::new(&render_program, "iResolution").with_2f(cw as f32, ch as f32);

    let post_vert_source =
    "
    #version 450 core
    layout (location = 0) in vec3 Position;

    out vec2 uv;

    void main()
    {
        uv = Position.xy * 0.5 + vec2(0.5);
        gl_Position = vec4(Position, 1.0);
    }";

    let post_vert_shader = Shader::from_vert_source(&CString::new(post_vert_source).expect("Frag: could not create vertex c string."))
        .expect("Frag: could not create vertex shader.");

    let post_frag_source =
    "
    #version 450 core
    in vec2 uv;
    uniform sampler2D tex;

    out vec4 color;

    void main()
    {
        color = texture(tex, uv);
    }";
    let post_frag_shader = Shader::from_frag_source(&CString::new(post_frag_source).expect("Frag: could not create fragment c string."))
        .expect("Frag: could not create fragment shader.");

    let post_program = Program::from_shaders(&[post_vert_shader, post_frag_shader]).expect("Frag: could not create shader program.");

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
        gl::EnableVertexAttribArray(0); // this is "layout (location = 0)" in vertex shader
        gl::VertexAttribPointer(
            0,         // index of the generic vertex attribute ("layout (location = 0)")
            3,         // the number of components per generic vertex attribute
            gl::FLOAT, // data type
            gl::FALSE, // normalized (int-to-float conversion)
            (3 * std::mem::size_of::<f32>()) as gl::types::GLint, // stride (byte offset between consecutive attributes)
            std::ptr::null(),                                     // offset of the first component
        );
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::BindVertexArray(0);
    }

    let mut canvas_fbo: gl::types::GLuint = 0;
    let mut canvas_tex: gl::types::GLuint = 0;

    unsafe{
        gl::GenFramebuffers(1, &mut canvas_fbo);
        gl::BindFramebuffer(gl::FRAMEBUFFER, canvas_fbo);
        gl::GenTextures(1, &mut canvas_tex);
        gl::BindTexture(gl::TEXTURE_2D, canvas_tex);

        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGB as i32, cw as i32, ch as i32, 0, gl::RGB, gl::UNSIGNED_BYTE, std::ptr::null());
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

        gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, canvas_tex, 0);
        if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE{
            panic!("Frag: could not initialize canvas framebuffer.");
        }
    }

    let mut t = 0.0;
    let mut dt = 0.0;
    let mut frame = 0;
    let mut event_pump = sdl_context.event_pump().unwrap();
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
        unsafe{
            gl::BindFramebuffer(gl::FRAMEBUFFER, canvas_fbo);
            render_program.set_used();
            gl::Viewport(0, 0, cw as i32, ch as i32);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::BindVertexArray(vao);
            i_time.set_1f(t);
            i_delta_time.set_1f(dt);
            i_frame.set_1ui(frame);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            post_program.set_used();
            gl::Viewport(0, 0, ww as i32, wh as i32);
            gl::BindTexture(gl::TEXTURE_2D, canvas_tex);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
        }
        window.gl_swap_window();

        frame += 1;
        t = start.elapsed().as_millis() as f32 / 1000.0;
        dt = t - lt;
        print!("{}, ", dt);
    }

    unsafe{
        gl::DeleteFramebuffers(1, &canvas_fbo);
    }
}

// Hello world opengl code stolen from here
// https://nercury.github.io/rust/opengl/tutorial/2018/02/10/opengl-in-rust-from-scratch-03-compiling-shaders.html
// https://nercury.github.io/rust/opengl/tutorial/2018/02/11/opengl-in-rust-from-scratch-04-triangle.html

pub struct Program {
    id: gl::types::GLuint,
}

impl Program {
    pub fn from_shaders(shaders: &[Shader]) -> Result<Program, String> {
        let program_id = unsafe { gl::CreateProgram() };

        for shader in shaders {
            unsafe {
                gl::AttachShader(program_id, shader.id());
            }
        }

        unsafe {
            gl::LinkProgram(program_id);
        }

        let mut success: gl::types::GLint = 1;
        unsafe {
            gl::GetProgramiv(program_id, gl::LINK_STATUS, &mut success);
        }

        if success == 0 {
            let mut len: gl::types::GLint = 0;
            unsafe {
                gl::GetProgramiv(program_id, gl::INFO_LOG_LENGTH, &mut len);
            }

            let error = create_whitespace_cstring_with_len(len as usize);

            unsafe {
                gl::GetProgramInfoLog(
                    program_id,
                    len,
                    std::ptr::null_mut(),
                    error.as_ptr() as *mut gl::types::GLchar,
                );
            }

            return Err(error.to_string_lossy().into_owned());
        }

        for shader in shaders {
            unsafe {
                gl::DetachShader(program_id, shader.id());
            }
        }

        Ok(Program { id: program_id })
    }

    pub fn id(&self) -> gl::types::GLuint {
        self.id
    }

    pub fn set_used(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}

pub struct Shader {
    id: gl::types::GLuint,
}

impl Shader {
    pub fn from_source(source: &CStr, kind: gl::types::GLenum) -> Result<Shader, String> {
        let id = shader_from_source(source, kind)?;
        Ok(Shader { id })
    }

    pub fn from_vert_source(source: &CStr) -> Result<Shader, String> {
        Shader::from_source(source, gl::VERTEX_SHADER)
    }

    pub fn from_frag_source(source: &CStr) -> Result<Shader, String> {
        Shader::from_source(source, gl::FRAGMENT_SHADER)
    }

    pub fn id(&self) -> gl::types::GLuint {
        self.id
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteShader(self.id);
        }
    }
}

fn shader_from_source(source: &CStr, kind: gl::types::GLenum) -> Result<gl::types::GLuint, String> {
    let id = unsafe { gl::CreateShader(kind) };
    unsafe {
        gl::ShaderSource(id, 1, &source.as_ptr(), std::ptr::null());
        gl::CompileShader(id);
    }

    let mut success: gl::types::GLint = 1;
    unsafe {
        gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut success);
    }

    if success == 0 {
        let mut len: gl::types::GLint = 0;
        unsafe {
            gl::GetShaderiv(id, gl::INFO_LOG_LENGTH, &mut len);
        }

        let error = create_whitespace_cstring_with_len(len as usize);

        unsafe {
            gl::GetShaderInfoLog(
                id,
                len,
                std::ptr::null_mut(),
                error.as_ptr() as *mut gl::types::GLchar,
            );
        }

        return Err(error.to_string_lossy().into_owned());
    }

    Ok(id)
}

fn create_whitespace_cstring_with_len(len: usize) -> CString {
    // allocate buffer of correct size
    let mut buffer: Vec<u8> = Vec::with_capacity(len + 1);
    // fill it with len spaces
    buffer.extend([b' '].iter().cycle().take(len));
    // convert buffer to CString
    unsafe { CString::from_vec_unchecked(buffer) }
}

struct Uniform{
    loc: gl::types::GLint,
}

impl Uniform{
    fn new(program: &Program, name: &str) -> Self{
        let cname = CString::new(name).expect("expected uniform name to have no nul bytes");
        let loc = unsafe {
            gl::GetUniformLocation(program.id(), cname.as_bytes_with_nul().as_ptr() as *const i8)
        };

        Self{ loc }
    }

    fn set_1f(&self, v: f32){ unsafe{ gl::Uniform1f(self.loc, v); } }
    fn with_1f(self, v: f32) -> Self{ self.set_1f(v); self }

    fn set_1ui(&self, v: u32){ unsafe{ gl::Uniform1ui(self.loc, v); } }
    fn with_1ui(self, v: u32) -> Self{ self.set_1ui(v); self }

    fn set_2f(&self, x: f32, y: f32){ unsafe{ gl::Uniform2f(self.loc, x, y); } }
    fn with_2f(self, x: f32, y: f32) -> Self{ self.set_2f(x, y); self }
}



#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
