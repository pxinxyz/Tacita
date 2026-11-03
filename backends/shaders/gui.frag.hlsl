#ifndef GUI_MANAGED_SRGB
#define GUI_MANAGED_SRGB 0
#endif
float4 guiLinearColor(float4 color) {
#if GUI_MANAGED_SRGB
 color.rgb=float3(color.r<=0.04045?color.r/12.92:pow((color.r+0.055)/1.055,2.4),color.g<=0.04045?color.g/12.92:pow((color.g+0.055)/1.055,2.4),color.b<=0.04045?color.b/12.92:pow((color.b+0.055)/1.055,2.4));
#endif
 return color;
}
struct Stop { float4 position; float4 color; };
struct Mask { float4 rect; float4 params; float4 kind; };
cbuffer effectsBuffer : register(b1) {
    float4 paintRect; float4 paintGeometry; float4 paintParameters; float4 maskCount;
    Stop stops[16]; Mask masks[4];
    float4 sdfRect; float4 sdfRadii; float4 sdfParameters; float4 sdfLight;
    float4 surfaceRect;float4 surfaceControl;float4 surfaceNoise;float4 surfaceGrade;
    float4 surfaceTint;float4 surfacePattern;float4 surfaceLight;float4 surfaceDistort;
    float4 filterControl;float4 filterValues;float4 filterExtra[2];
};
struct PS_INPUT {
    float4 pos : SV_POSITION; float4 col : COLOR0;
    float2 uv : TEXCOORD0; float2 logical : TEXCOORD1;
};
sampler sampler0 : register(s0);
Texture2D texture0 : register(t0);
Texture2D mask0 : register(t1);
Texture2D mask1 : register(t2);
Texture2D mask2 : register(t3);
Texture2D mask3 : register(t4);
float spread(float value, float mode) {
    if (mode < 0.5) return saturate(value);
    if (mode < 1.5) return frac(value);
    float p = value - floor(value / 2) * 2;
    return p <= 1 ? p : 2 - p;
}
float4 paint(float2 pos) {
    if (paintParameters.x < 0.5) return float4(1,1,1,1);
    float2 p = (pos - paintRect.xy) / (paintRect.zw - paintRect.xy) - paintGeometry.xy;
    float2 v = paintGeometry.zw;
    float t;
    if (paintParameters.x < 1.5) t = dot(p,v) / dot(v,v);
    else if (paintParameters.x < 2.5) t = length(p / v);
    else t = frac((atan2(p.y,p.x) - atan2(v.y,v.x)) / 6.28318530718 + 1);
    t = spread(t + paintParameters.z, paintParameters.y);
    int count = min((int)paintParameters.w,16);
    for (int i=1; i<count; ++i) {
        if (t <= stops[i].position.x) {
            float f = saturate((t-stops[i-1].position.x)/(stops[i].position.x-stops[i-1].position.x));
            return lerp(guiLinearColor(stops[i-1].color),guiLinearColor(stops[i].color),f);
        }
    }
    return guiLinearColor(stops[max(count-1,0)].color);
}
float sampleMask(int index, float2 uv) {
    if (index == 0) return mask0.SampleLevel(sampler0,uv,0).a;
    if (index == 1) return mask1.SampleLevel(sampler0,uv,0).a;
    if (index == 2) return mask2.SampleLevel(sampler0,uv,0).a;
    return mask3.SampleLevel(sampler0,uv,0).a;
}

// Circular per-corner SDF and its outward normal, in logical pixels.
float3 shapeDistance(float2 pos) {
    float2 p = pos - (sdfRect.xy + sdfRect.zw) * 0.5;
    float radius = p.y < 0 ? (p.x < 0 ? sdfRadii.x : sdfRadii.y)
                               : (p.x < 0 ? sdfRadii.w : sdfRadii.z);
    float2 q = abs(p) - (sdfRect.zw - sdfRect.xy) * 0.5 + radius;
    float2 outside = max(q,0);
    float len = length(outside);
    float2 normal = len > 0.000001 ? outside / max(len,0.000001)
        : (q.x > q.y ? float2(1,0) : float2(0,1));
    normal *= float2(p.x < 0 ? -1 : 1, p.y < 0 ? -1 : 1);
    return float3(len + min(max(q.x,q.y),0) - radius, normal);
}
float covered(float distance, float aa) {
    return sdfLight.w > 0.5 ? saturate(0.5 - distance / aa) : (distance <= 0 ? 1 : 0);
}
float shapeCoverage(float2 pos) {
    float mode = sdfParameters.x;
    if (mode < 0.5) return 1;
    float3 distanceNormal = shapeDistance(pos);
    float d = distanceNormal.x;
    // Euclidean derivative length gives one physical pixel at every orientation.
    float aa = max(length(float2(ddx(d),ddy(d))),0.000001);
    if (mode < 1.5) return covered(d,aa);
    if (mode < 2.5) {
        float width = sdfParameters.y;
        float high = sdfParameters.z < 0.5 ? 0 : (sdfParameters.z < 1.5 ? width * 0.5 : width);
        return max(covered(d-high,aa)-covered(d-high+width,aa),0);
    }
    if (mode < 4.5) {
        float reach = max(sdfParameters.w,0.000001);
        float distance = mode < 3.5 ? max(d,0) : abs(d);
        // Subtract the cutoff value: zero at six reaches, no rectangular seam.
        return saturate((exp(-distance/reach)-exp(-6.0))/(1-exp(-6.0)));
    }
    float band = max(covered(d,aa)-covered(d+sdfParameters.y,aa),0);
    return band * pow(saturate(dot(distanceNormal.yz,sdfLight.xy)),sdfLight.z);
}
float reveal(Mask m, float2 pos) {
    float2 p = (pos-m.rect.xy)/(m.rect.zw-m.rect.xy);
    float t = p.x;
    if (m.kind.x < 3.5) t = p.x;
    else if (m.kind.x < 4.5) t = 1-p.x;
    else if (m.kind.x < 5.5) t = p.y;
    else if (m.kind.x < 6.5) t = 1-p.y;
    else t = length((p-0.5)*2)/1.41421356237;
    float progress = m.params.x;
    float a;
    if (progress <= 0) a = 0;
    else if (progress >= 1) a = 1;
    else if (m.params.y <= 0) a = t <= progress ? 1 : 0;
    else {
        // Feather starts outside at progress=0 and ends outside at progress=1.
        float edge = lerp(-m.params.y*0.5,1+m.params.y*0.5,progress);
        a = 1-smoothstep(edge-m.params.y*0.5,edge+m.params.y*0.5,t);
    }
    a = m.params.z > 0.5 ? 1-a : a;
    return all(p >= 0) && all(p <= 1) ? a : 0;
}

float4 filtered(float2 uv);
float4 guiInputColor(float4 color, uint bit) {
    if (((uint)maskCount.y & bit)!=0) color.rgb = color.a>0 ? color.rgb/color.a : 0;
    return color;
}
float2 warped(float2 uv,float2 pos);
float4 surfaceColor(float4 input,float2 uv,float2 pos,float4 modulation);
float4 main(PS_INPUT input) : SV_Target {
    if (maskCount.z>0.5) clip(shapeCoverage(input.logical)-0.5);
    float2 uv=warped(input.uv,input.logical);
    float4 painting=paint(input.logical);
    float4 color = surfaceColor(input.col*filtered(uv)*painting,uv,input.logical,input.col*painting);
    color.a *= shapeCoverage(input.logical);
    int count = min((int)maskCount.x,4);
    for (int i=0; i<count; ++i) {
        Mask m = masks[i];
        if (m.kind.x < 1.5) {
            float2 p = (input.logical-m.rect.xy)/(m.rect.zw-m.rect.xy);
            float inside = all(p >= 0) && all(p <= 1) ? 1 : 0;
            color.a *= inside * sampleMask(i,lerp(m.params.xy,m.params.zw,saturate(p)));
        } else if (m.kind.x < 2.5) {
            float2 d = m.rect.zw-m.rect.xy;
            float t = saturate(dot(input.logical-m.rect.xy,d)/dot(d,d));
            color.a *= lerp(m.params.x,m.params.y,t);
        } else {
            color.a *= reveal(m,input.logical);
        }
    }
    if (((uint)maskCount.y & 2)!=0) color.rgb *= color.a;
    return color;
}
