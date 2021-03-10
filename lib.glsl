float circle(vec2 uv, vec2 pos, float rad, float blur){
    return smoothstep(rad, rad - blur, length(uv - pos));
}
