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

const vec3 offsets = vec3(3.141592654) * vec3(1.0 / 2.0, 1.0 / 2.0 - 2.0 / 3.0, 1.0 / 2.0 - 4.0 / 3.0);

void main()
{
    vec2 uv = u_uv.xy + v_uv * u_uv.zw;
    vec2 f = fract(uv * TextureSize);
    vec3 c = texture2D(Texture, uv).rgb;
    bool inside = all(greaterThanEqual(uv, u_pic.xy)) && all(lessThan(uv, u_pic.zw));
    if (inside) {
        vec2 angle = f * 6.283185307;
        float yfactor = (16.0 + sin(angle.y)) / 17.0;
        vec3 xfactors = (4.0 + sin(angle.x + offsets)) / 5.0;
        c *= yfactor * xfactors;
    }
    FRAG_COLOR = vec4(c, 1.0);
}

#endif
