#include <metal_stdlib>
using namespace metal;

struct VertexOut {
    float4 position [[position]];
    float2 texCoord;
};

vertex VertexOut vertexShader(uint vertexID [[vertex_id]]) {
    float2 positions[4] = {
        float2(-1, -1),
        float2( 1, -1),
        float2(-1,  1),
        float2( 1,  1)
    };
    float2 texCoords[4] = {
        float2(0, 1),
        float2(1, 1),
        float2(0, 0),
        float2(1, 0)
    };
    VertexOut out;
    out.position = float4(positions[vertexID], 0, 1);
    out.texCoord = texCoords[vertexID];
    return out;
}

fragment float4 fragmentShader(VertexOut in [[stage_in]],
                                texture2d<float> yTexture [[texture(0)]],
                                texture2d<float> cbcrTexture [[texture(1)]]) {
    constexpr sampler s(address::clamp_to_edge, filter::linear);
    
    float y = yTexture.sample(s, in.texCoord).r;
    float2 cbcr = cbcrTexture.sample(s, in.texCoord).rg;
    
    float cb = cbcr.x - 0.5;
    float cr = cbcr.y - 0.5;
    
    // BT.709 conversion
    float r = y + 1.5748 * cr;
    float g = y - 0.1873 * cb - 0.4681 * cr;
    float b = y + 1.8556 * cb;
    
    return float4(r, g, b, 1.0);
}
