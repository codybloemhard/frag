void main()
{
    float rad = 0.4 + (sin(iTime * 10.0) * 0.5 + 0.5) * 0.1;
    color = vec4(1.0f, 0.5f, 0.2f, 1.0f) * circle(uv, vec2(0.0), rad, 0.05);
}
