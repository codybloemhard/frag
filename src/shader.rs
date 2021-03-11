use std::ffi::{ CStr, CString };
use std::fs::File;
use std::io::Read;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use hotwatch::{ Hotwatch, Event };

static STREAM_FLAG: AtomicU32 = AtomicU32::new(0);

pub const RENDER_VERT_SRC: &str = "
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

pub const RENDER_FRAG_HEADER: &str = "
#version 450 core
in vec2 uv;
uniform float iTime;
uniform float iAspect;
uniform vec2 iResolution;

out vec4 color;
";

pub const RENDER_FRAG_STD_BODY: &str = "
void main()
{
    float rad = 0.4 + (sin(iTime) * 0.5 + 0.5) * 0.1;
    color = vec4(1.0f, 0.5f, 0.2f, 1.0f) * smoothstep(rad, rad-0.001, length(uv));
}
";

pub const POST_VERT_SRC: &str = "
#version 450 core
layout (location = 0) in vec3 Position;

out vec2 uv;

void main()
{
    uv = Position.xy * 0.5 + vec2(0.5);
    gl_Position = vec4(Position, 1.0);
}";

pub const POST_FRAG_SRC: &str = "
#version 450 core
in vec2 uv;
uniform sampler2D tex;

out vec4 color;

void main()
{
    color = texture(tex, uv);
}";

#[derive(Debug)]
enum StreamElement {
    Static(String),
    Streamed(String),
}

#[derive(Debug)]
pub struct ShaderStreamer{
    segments: Vec<StreamElement>,
    hotwatch: Hotwatch,
    stream_cache: Vec<String>,
}

impl ShaderStreamer{
    pub fn new() -> Self{
        let hotwatch = Hotwatch::new().expect("hotwatch failed to initialize!");
        Self{
            segments: vec![StreamElement::Static(RENDER_FRAG_HEADER.to_string())],
            hotwatch,
            stream_cache: Vec::new(),
        }
    }

    pub fn test() -> Self{
        Self::new().with_str(RENDER_FRAG_STD_BODY)
    }

    pub fn with_str(mut self, string: &str) -> Self{
        self.segments.push(StreamElement::Static(string.to_string()));
        self
    }

    pub fn with_file(mut self, file: &str) -> Self{
        self.segments.push(StreamElement::Streamed(file.to_string()));
        self.stream_cache.push(String::new());
        self
    }

    fn read_file(file: &str) -> Result<String, String>{
        let mut file = match File::open(file){
            Ok(f) => f,
            Err(e) => { return Err(format!("{}", e)); }
        };
        let mut contents = String::new();
        if let Err(e) = file.read_to_string(&mut contents){
            return Err(format!("{}", e));
        }
        Ok(contents)
    }

    pub fn start(&mut self){
        let mut stream_count = 0u32;
        for element in &self.segments{
            if let StreamElement::Streamed(file) = element{
                self.hotwatch.watch(&file, move |event: Event| {
                    if let Event::Write(path) = event {
                        println!("Frag: marked \"{:?}\" dirty.", path.to_str());
                        let flag = 1u32 << stream_count;
                        STREAM_FLAG.fetch_or(flag, Ordering::SeqCst);
                    }
                }).expect("failed to watch file!");
                stream_count += 1;
            }
        }
    }

    pub fn is_dirty(&self) -> bool{
        STREAM_FLAG.load(Ordering::SeqCst) > 0
    }

    pub fn build(&mut self, all: bool) -> Result<Program, String>{
        let flag = STREAM_FLAG.load(Ordering::Relaxed);
        if self.stream_cache.len() > 31 {
            return Err("Can only support up to 31 streamed files.".to_string());
        }
        let mut concat = String::new();
        let mut stream_count = 0;
        for element in &self.segments{
            match element{
                StreamElement::Streamed(file) => {
                    let bit = 1u32 << stream_count;
                    if all || (bit & flag) > 0{
                        let content = Self::read_file(&file)?;
                        concat.push_str(&content);
                        self.stream_cache[stream_count] = content;
                        if !all {
                            STREAM_FLAG.fetch_xor(bit, Ordering::SeqCst);
                        }
                    } else {
                        concat.push_str(&self.stream_cache[stream_count]);
                    }
                    stream_count += 1;
                },
                StreamElement::Static(string) => {
                    concat.push_str(&string);
                },
            }
        }
        Program::new(RENDER_VERT_SRC, &concat)
    }
}

impl Default for ShaderStreamer{
    fn default() -> Self {
        Self::new()
    }
}

pub struct Program {
    id: gl::types::GLuint,
}

impl Program {
    pub fn new(vert_source: &str, frag_source: &str) -> Result<Self, String>{
        let vert_cstr = if let Ok(cstr) = CString::new(vert_source){ cstr } else { return Err("Frag: could not make vert cstr.".to_string()); };
        let frag_cstr = if let Ok(cstr) = CString::new(frag_source){ cstr } else { return Err("Frag: could not make frag cstr.".to_string()); };
        let vert_shader = Shader::from_vert_source(&vert_cstr)?;
        let frag_shader = Shader::from_frag_source(&frag_cstr)?;

        Program::from_shaders(&[vert_shader, frag_shader])
    }

    pub fn from_shaders(shaders: &[Shader]) -> Result<Self, String> {
        let id = unsafe {
            let id = gl::CreateProgram();
            for shader in shaders {
                gl::AttachShader(id, shader.id());
            }
            gl::LinkProgram(id);
            let mut success: gl::types::GLint = 1;
            gl::GetProgramiv(id, gl::LINK_STATUS, &mut success);
            if success == 0 {
                let mut len: gl::types::GLint = 0;
                gl::GetProgramiv(id, gl::INFO_LOG_LENGTH, &mut len);
                let error = create_whitespace_cstring_with_len(len as usize);
                gl::GetProgramInfoLog(id, len, std::ptr::null_mut(), error.as_ptr() as *mut gl::types::GLchar);
                return Err(error.to_string_lossy().into_owned());
            }
            for shader in shaders {
                gl::DetachShader(id, shader.id());
            }
            id
        };
        Ok(Program { id })
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
            gl::GetShaderInfoLog(id, len, std::ptr::null_mut(), error.as_ptr() as *mut gl::types::GLchar);
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

pub struct Uniform{
    loc: gl::types::GLint,
    cname: CString,
}

impl Uniform{
    pub fn new(program: &Program, name: &str) -> Self{
        let cname = CString::new(name).expect("expected uniform name to have no nul bytes");
        let loc = unsafe {
            gl::GetUniformLocation(program.id(), cname.as_bytes_with_nul().as_ptr() as *const i8)
        };

        Self{
            loc,
            cname,
        }
    }

    pub fn reload(&mut self, program: &Program){
        let loc = unsafe {
            gl::GetUniformLocation(program.id(), self.cname.as_bytes_with_nul().as_ptr() as *const i8)
        };
        self.loc = loc;
    }

    pub fn set_1f(&self, v: f32){ unsafe{ gl::Uniform1f(self.loc, v); } }
    pub fn with_1f(self, v: f32) -> Self{ self.set_1f(v); self }

    pub fn set_1ui(&self, v: u32){ unsafe{ gl::Uniform1ui(self.loc, v); } }
    pub fn with_1ui(self, v: u32) -> Self{ self.set_1ui(v); self }

    pub fn set_2f(&self, x: f32, y: f32){ unsafe{ gl::Uniform2f(self.loc, x, y); } }
    pub fn with_2f(self, x: f32, y: f32) -> Self{ self.set_2f(x, y); self }
}
