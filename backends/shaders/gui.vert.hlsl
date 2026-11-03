#ifndef GUI_MANAGED_SRGB
#define GUI_MANAGED_SRGB 0
#endif
float4 guiLinearColor(float4 color) {
#if GUI_MANAGED_SRGB
 color.rgb=float3(color.r<=0.04045?color.r/12.92:pow((color.r+0.055)/1.055,2.4),color.g<=0.04045?color.g/12.92:pow((color.g+0.055)/1.055,2.4),color.b<=0.04045?color.b/12.92:pow((color.b+0.055)/1.055,2.4));
#endif
 return color;
}
cbuffer vertexBuffer : register(b0) { float4x4 ProjectionMatrix; };
struct VS_INPUT { float2 pos : POSITION; float2 uv : TEXCOORD0; float4 col : COLOR0; };
struct PS_INPUT {
    float4 pos : SV_POSITION; float4 col : COLOR0;
    float2 uv : TEXCOORD0; float2 logical : TEXCOORD1;
};
PS_INPUT main(VS_INPUT input) {
    PS_INPUT output;
    output.pos = mul(ProjectionMatrix, float4(input.pos, 0, 1));
    output.col = guiLinearColor(input.col); output.uv = input.uv; output.logical = input.pos;
    return output;
}
struct QUAD_INPUT {
    float2 p0 : POSITION0; float2 uv0 : TEXCOORD0; float4 c0 : COLOR0;
    float2 p1 : POSITION1; float2 uv1 : TEXCOORD1; float4 c1 : COLOR1;
    float2 p2 : POSITION2; float2 uv2 : TEXCOORD2; float4 c2 : COLOR2;
    float2 p3 : POSITION3; float2 uv3 : TEXCOORD3; float4 c3 : COLOR3;
};
PS_INPUT quad_main(QUAD_INPUT input, uint vertex : SV_VertexID) {
    uint corners[6] = {0,1,2,0,2,3};
    uint corner = corners[vertex];
    VS_INPUT v;
    v.pos = corner==0 ? input.p0 : corner==1 ? input.p1 : corner==2 ? input.p2 : input.p3;
    v.uv = corner==0 ? input.uv0 : corner==1 ? input.uv1 : corner==2 ? input.uv2 : input.uv3;
    v.col = corner==0 ? input.c0 : corner==1 ? input.c1 : corner==2 ? input.c2 : input.c3;
    return main(v);
}
