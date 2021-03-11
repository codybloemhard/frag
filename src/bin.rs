use frag::*;

fn main(){
    let mul = 1;
    let cw = 320*mul;
    let ch = 180*mul;
    let ww = 1600;
    let wh = 900;
    println!("Henlo Frens!");

    let builder = shader::ShaderStreamer::new()
        .with_file("lib.glsl")
        .with_file("shader.glsl");
    //run(cw, ch, ww, wh, true, builder);
    render(cw, ch, ww, wh, true, builder);
}
