use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;
use std::time::Instant;
use std::ffi::{CStr, CString};

pub fn run(cw: i32, ch: i32, ww: u32, wh: u32, sleep: u32) {
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

    unsafe{
        gl::Viewport(0, 0, ww as i32, wh as i32);
        gl::ClearColor(0.3, 0.3, 0.5, 1.0);
    }

    let vert_source =
    "
    #version 330 core
    layout (location = 0) in vec3 Position;
    out vec2 uv;
    void main()
    {
        uv = Position.xy * 0.5;
        uv.x *= 16./9.;
        gl_Position = vec4(Position, 1.0);
    }";

    let vert_shader = Shader::from_vert_source(&CString::new(vert_source).expect("Frag: could not create vertex c string."))
        .expect("Frag: could not create vertex shader.");

    let frag_source =
    "
    #version 330 core
    in vec2 uv;
    out vec4 Color;
    void main()
    {
        Color = vec4(1.0f, 0.5f, 0.2f, 1.0f) * smoothstep(0.5, 0.5-0.001, length(uv - vec2(0.0)));
    }";
    let frag_shader = Shader::from_frag_source(&CString::new(frag_source).expect("Frag: could not create fragment c string."))
        .expect("Frag: could not create fragment shader.");

    let shader_program = Program::from_shaders(&[vert_shader, frag_shader]).expect("Frag: could not create shader program.");
    shader_program.set_used();

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

    let mut event_pump = sdl_context.event_pump().unwrap();
    let start = Instant::now();
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. }
                    => { break 'running  },
                _ => {}
            }
        }
        unsafe{
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::BindVertexArray(vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
        }
        window.gl_swap_window();
        let t = start.elapsed().as_millis() as f32 / 1000.0;
        print!("{}, ", t);
        std::thread::sleep(Duration::new(0, sleep / 60));
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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
