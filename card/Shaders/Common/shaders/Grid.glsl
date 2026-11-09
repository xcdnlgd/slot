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
uniform vec2 OutputSize;
uniform vec4 u_uv;
uniform vec4 u_pic;
varying vec2 v_uv;

vec2 grid_band(vec2 x)
{
    return floor(x) / 3.0 + min(fract(x), vec2(1.0 / 3.0));
}

vec4 whole_scale(vec2 at, vec2 len)
{
    vec2 shown = len * TextureSize;
    vec2 scale = ceil(OutputSize / shown);
    vec2 trim = scale * shown - OutputSize;
    vec2 keep = step(0.1 * shown, trim);
    vec2 top = floor(trim * 0.5);
    vec2 new_at = at + (vec2(1.0) - keep) * top / scale / TextureSize;
    vec2 new_len = mix(OutputSize / scale / TextureSize, len, keep);
    return vec4(new_at, new_len);
}

void main()
{
    vec4 src = whole_scale(u_uv.xy, u_uv.zw);
    vec2 uv = src.xy + v_uv * src.zw;
    vec2 f = fract(uv * TextureSize);
    vec3 c = texture2D(Texture, uv).rgb;
    bool inside = all(greaterThanEqual(uv, u_pic.xy)) && all(lessThan(uv, u_pic.zw));
    if (inside) {
        vec2 hp = 0.5 * src.zw * TextureSize / OutputSize;
        vec2 edge = (grid_band(f + hp) - grid_band(f - hp)) / (2.0 * hp);
        c *= 1.0 - 0.75 * max(edge.x, edge.y);
    }
    FRAG_COLOR = vec4(c, 1.0);
}

#endif
