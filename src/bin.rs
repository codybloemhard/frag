use frag::*;

fn main(){
    let mul = 1;
    let cw = 320*mul;
    let ch = 180*mul;
    let ww = 1600;
    let wh = 900;
    println!("Henlo Frens!");
    run(cw, ch, ww, wh, 1_000_000_000u32);
}
