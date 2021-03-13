use frag::*;

fn main(){
    let mul = 1;
    println!("Henlo Frens!");

    let streamer = shader::ShaderStreamer::new()
        .with_file("lib.glsl")
        .with_file("shader.glsl");
    FragConf::new()
        .with_window_width(1600)
        .with_window_height(900)
        .with_canvas_width(320 * mul)
        .with_canvas_height(180 * mul)
        .with_pixelate(true)
        .with_streamer(streamer)
        .run_live().expect("Could not run.");
        // .into_ffmpeg_renderer()
        // .with_framerate(30)
        // .with_crf(20)
        // .with_preset(Preset::Slow)
        // .with_tune(Tune::Animation)
        // .with_length(600)
        // .with_output("render.mp4")
        // .render().expect("Could not render.");
}
