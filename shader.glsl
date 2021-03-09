void main()
{
    float rad = 0.4 + (sin(iTime * 2.0) * 0.5 + 0.5) * 0.1;
    color = vec4(1.0f, 0.5f, 0.2f, 1.0f) * smoothstep(rad, rad-0.001, length(uv - vec2(0.0)));
}
