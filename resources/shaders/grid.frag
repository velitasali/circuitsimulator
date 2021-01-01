// Circuit background: viewport fill, canvas paper and the main grid dots.
//
// This used to be CPU work in Circuit::drawBackground(), where the dot count
// grows with the square of the visible area -- zoomed out far enough it was
// hundreds of thousands of individually blended points per frame. As a shader
// it costs one pass over the visible pixels no matter the zoom, so it is
// effectively free and, unlike the painted version, it does not have to be
// redrawn when the canvas pans.
//
// The finer 0.5/0.25 sub-grids stay on the CPU: they only appear at zoom >= 2,
// where the visible area is small enough that their cost is negligible anyway.

#version 440

layout(location = 0) in vec2 qt_TexCoord0;
layout(location = 0) out vec4 fragColor;

layout(std140, binding = 0) uniform buf {
    mat4 qt_Matrix;
    float qt_Opacity;
    vec2 itemSize;      // Item size, logical px
    vec2 center;        // Scene coords at the item's centre
    vec4 sceneRect;     // Circuit bounds, scene coords (x, y, w, h)
    vec4 gridColor;     // Straight (non-premultiplied) RGBA
    vec4 canvasColor;
    vec4 viewportColor;
    float zoom;
    float dpr;          // Device pixel ratio, so the dots stay one pixel crisp
    float showGrid;
};

// Smallest "nice" multiple (1, 2, 5, 10, 20, 50, ...) >= v. Mirrors the
// niceSteps walk in Circuit::drawBackground() so the two agree on spacing.
float niceCeil( float v ) {
    if ( v <= 1.0 ) return 1.0;
    float e = floor( log( v ) / log( 10.0 ) );
    float p = pow( 10.0, e );
    float m = v / p;
    float nm = m <= 1.0 ? 1.0 : ( m <= 2.0 ? 2.0 : ( m <= 5.0 ? 5.0 : 10.0 ) );
    return nm * p;
}

void main() {
    vec2 px = qt_TexCoord0 * itemSize;
    vec2 scene = center + ( px - itemSize * 0.5 ) / zoom;

    // Soft-edged test against the circuit's paper bounds, one device pixel wide,
    // so the boundary reads as crisp rather than aliased at any zoom level.
    vec2 rectMin = sceneRect.xy;
    vec2 rectMax = sceneRect.xy + sceneRect.zw;
    float px1 = 1.0 / max( zoom * dpr, 0.0001 );
    vec2 edgeLo = smoothstep( rectMin - px1, rectMin, scene );
    vec2 edgeHi = 1.0 - smoothstep( rectMax, rectMax + px1, scene );
    float inside = edgeLo.x * edgeLo.y * edgeHi.x * edgeHi.y;

    vec4 base = mix( viewportColor, canvasColor, inside );
    vec4 outc = vec4( base.rgb * base.a, base.a ); // premultiplied

    if ( showGrid > 0.5 && inside > 0.0 ) {
        // Main grid lattice: x = 8n + 4, y = 8n + 4 in scene units.
        float k = niceCeil( 7.0 / ( 8.0 * zoom ) );
        float stepScene = 8.0 * k;

        // Distance in device px from this fragment to the nearest main lattice point.
        vec2 g = ( scene - vec2( 4.0 ) ) / stepScene;
        vec2 d = abs( fract( g + 0.5 ) - 0.5 ) * stepScene * zoom * dpr;
        float dist = length( d );

        float r = max( 0.75 * dpr, 0.75 * zoom * dpr ); // Scales with zoom so dots maintain proportion to grid cells
        float cov = ( 1.0 - smoothstep( r - 0.4, r + 0.4, dist ) ) * gridColor.a * inside;

        // Fractional grid at 0.5 (x = 8n + 8, y = 8n + 8) when zoom >= 2.0
        if ( zoom >= 2.0 ) {
            vec2 g5 = ( scene - vec2( 8.0 ) ) / 8.0;
            vec2 d5 = abs( fract( g5 + 0.5 ) - 0.5 ) * 8.0 * zoom * dpr;
            float dist5 = length( d5 );
            float r5 = max( 0.5 * dpr, 0.5 * zoom * dpr );
            float cov5 = ( 1.0 - smoothstep( r5 - 0.35, r5 + 0.35, dist5 ) ) * gridColor.a * 0.45 * inside;
            cov = max( cov, cov5 );
        }

        vec4 dot = vec4( gridColor.rgb * cov, cov );
        outc = dot + outc * ( 1.0 - cov );
    }

    fragColor = outc * qt_Opacity;
}
