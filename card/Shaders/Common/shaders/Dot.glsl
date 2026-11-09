#if defined(VERTEX)

attribute vec4 VertexCoord;
attribute vec4 TexCoord;
uniform vec2 OutputSize;
uniform vec4 u_rect;
varying vec2 v_uv;

void main()
{
    v_uv = TexCoord.xy;
    vec2 p = (u_rect.xy + TexCoord.xy * u_rect.zw) / OutputSize;
    gl_Position = vec4(p.x * 2.0 - 1.0, 1.0 - p.y * 2.0, 0.0, 1.0);
}

#elif defined(FRAGMENT)

#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif

uniform sampler2D Texture;
uniform sampler2D u_paper;
uniform vec2 TextureSize;
uniform float u_paper_size;
uniform vec2 u_origin;
uniform vec4 u_uv;
uniform vec4 u_pic;
varying vec2 v_uv;

const vec3 luma_coeff = vec3(0.2126, 0.7152, 0.0722);

void main()
{
    vec2 uv = u_uv.xy + v_uv * u_uv.zw;
    vec2 f = fract(uv * TextureSize);
    vec3 c = texture2D(Texture, uv).rgb;
    bool inside = all(greaterThanEqual(uv, u_pic.xy)) && all(lessThan(uv, u_pic.zw));
    if (inside) {
        c = pow(c, vec3(1.05));
        vec2 d = abs(f - 0.5);
        float x2 = max(d.x, d.y);
        x2 = x2 * x2;
        float x4 = x2 * x2;
        float w = 48.0 * (x4 - (8.0 / 3.0) * x4 * x2);
        w = w * (w + (1.0 - w) * 0.65) * 0.65;
        float luma = dot(c, luma_coeff);
        w = w * luma;
        c = mix(c, vec3(1.0), w);
        vec3 bg = vec3(texture2D(u_paper, (floor(gl_FragCoord.xy - u_origin) + 0.5) / u_paper_size).r) * c;
        c = mix(c, bg, dot(c, luma_coeff));
    }
    FRAG_COLOR = vec4(c, 1.0);
}

#endif
