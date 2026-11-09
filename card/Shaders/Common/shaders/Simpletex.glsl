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
uniform vec2 TextureSize;
uniform vec4 u_uv;
uniform vec4 u_pic;
varying vec2 v_uv;

float dot_weight(vec2 f, vec2 o, float bloom)
{
    return exp(-4.0 * length(f - (o + vec2(0.5))) * bloom);
}

void main()
{
    vec2 uv = u_uv.xy + v_uv * u_uv.zw;
    vec2 f = fract(uv * TextureSize);
    vec3 c = texture2D(Texture, uv).rgb;
    bool inside = all(greaterThanEqual(uv, u_pic.xy)) && all(lessThan(uv, u_pic.zw));
    if (inside) {
        float bloom = mix(1.05, 0.95, dot(c, vec3(0.30, 0.59, 0.11)));
        float mid = dot_weight(f, vec2(0.0), bloom);
        float sum = 0.0;
        for (int j = -1; j <= 1; j++) {
            for (int i = -1; i <= 1; i++) {
                sum += dot_weight(f, vec2(float(i), float(j)), bloom);
            }
        }
        c = mix(c, mix(1.2 * c * mid, c * sum, 0.85), 0.85);
    }
    FRAG_COLOR = vec4(c, 1.0);
}

#endif
