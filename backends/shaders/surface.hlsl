bool hasSurface(uint flag) {return ((uint)surfaceControl.x & flag)!=0;}
float hashPixel(float2 p) {return frac(sin(dot(p,float2(127.1,311.7)))*43758.5453);}
float4 weighted(float4 c) {return float4(c.rgb*c.a,c.a);}
float4 straight(float4 c) {return float4(c.rgb/max(c.a,0.000001),c.a);}
float4 imageLinear(float2 uv) {
    uint w,h;texture0.GetDimensions(w,h);int2 size=int2(w,h);
    float2 p=saturate(uv)*size-0.5;int2 base=(int2)floor(p);float2 f=frac(p);
    float4 a=guiInputColor(texture0.Load(int3(clamp(base,0,size-1),0)),1);
    float4 b=guiInputColor(texture0.Load(int3(clamp(base+int2(1,0),0,size-1),0)),1);
    float4 c=guiInputColor(texture0.Load(int3(clamp(base+int2(0,1),0,size-1),0)),1);
    float4 d=guiInputColor(texture0.Load(int3(clamp(base+int2(1,1),0,size-1),0)),1);
    return straight(lerp(lerp(weighted(a),weighted(b),f.x),lerp(weighted(c),weighted(d),f.x),f.y));
}
float4 secondaryLinear(float2 uv) {
    uint w,h;mask3.GetDimensions(w,h);int2 size=int2(w,h);
    float2 p=saturate(uv)*size-0.5;int2 base=(int2)floor(p);float2 f=frac(p);
    float4 a=guiInputColor(mask3.Load(int3(clamp(base,0,size-1),0)),4);
    float4 b=guiInputColor(mask3.Load(int3(clamp(base+int2(1,0),0,size-1),0)),4);
    float4 c=guiInputColor(mask3.Load(int3(clamp(base+int2(0,1),0,size-1),0)),4);
    float4 d=guiInputColor(mask3.Load(int3(clamp(base+int2(1,1),0,size-1),0)),4);
    return straight(lerp(lerp(weighted(a),weighted(b),f.x),lerp(weighted(c),weighted(d),f.x),f.y));
}
float4 filtered(float2 uv) {
    uint kind=(uint)filterControl.x;
    if (kind==0) {
        if (hasSurface(64) || hasSurface(4096) || hasSurface(8192)) return imageLinear(uv);
        return guiInputColor(texture0.SampleLevel(sampler0,uv,0),1);
    }
    uint w,h;texture0.GetDimensions(w,h);float2 texel=all(filterValues.yz>0) ? filterValues.yz : 1.0/float2(w,h);
    float radius=filterControl.z;
    if (kind==1 || kind==2) {
        float2 axis=kind==1 ? float2(1,0) : float2(0,1);
        float4 sum=0;float total=0;
        [unroll] for (int i=-8;i<=8;++i) {
            float weight=exp(-0.5*(i/2.6666667)*(i/2.6666667));
            sum+=weighted(imageLinear(uv+axis*texel*i*radius/8))*weight;total+=weight;
        }
        return straight(sum/total);
    }
    if (kind==3) {
        float2 d=texel*max(radius,0.5);
        return straight((weighted(imageLinear(uv))*4+
            weighted(imageLinear(uv+float2(-d.x,-d.y)))+
            weighted(imageLinear(uv+float2(d.x,-d.y)))+
            weighted(imageLinear(uv+float2(-d.x,d.y)))+weighted(imageLinear(uv+d)))/8);
    }
    if (kind==4) {
        float2 d=texel*max(radius,0.5);
        float4 sum=weighted(imageLinear(uv+float2(-d.x*2,0)))+
            weighted(imageLinear(uv+float2(d.x*2,0)))+
            weighted(imageLinear(uv+float2(0,-d.y*2)))+
            weighted(imageLinear(uv+float2(0,d.y*2)));
        sum+=(weighted(imageLinear(uv-d))+weighted(imageLinear(uv+d))+
            weighted(imageLinear(uv+float2(-d.x,d.y)))+
            weighted(imageLinear(uv+float2(d.x,-d.y))))*2;
        return straight(sum/12);
    }
    float4 c=imageLinear(uv);
    if (kind==5) {
        float peak=max(c.r,max(c.g,c.b));
        return float4(c.rgb*max(peak-filterControl.w,0)/max(peak,0.000001),c.a);
    }
    if (kind==6) {
        float4 original=secondaryLinear(uv);
        float3 glow=c.rgb*c.a*filterValues.x;
        float a=max(original.a,saturate(dot(glow,float3(0.2126,0.7152,0.0722))));
        return float4((original.rgb*original.a+glow)/max(a,0.000001),a);
    }
    return c;
}
float2 warped(float2 uv,float2 pos) {
    if (!hasSurface(4096) && !hasSurface(8192)) return uv;
    float2 p=(pos-surfaceRect.xy)/max(surfaceRect.zw-surfaceRect.xy,0.001);
    float t=surfaceControl.y;uint w,h;texture0.GetDimensions(w,h);
    float2 offset=0;
    if (hasSurface(4096)) offset+=sin(p.yx*surfaceDistort.y*6.2831853+t)*surfaceDistort.x;
    if (hasSurface(8192)) offset+=(p-0.5)*dot(p-0.5,p-0.5)*4*surfaceDistort.w;
    return saturate(uv+offset/float2(w,h));
}
float4 surfaceColor(float4 input,float2 uv,float2 pos,float4 modulation) {
    if (surfaceControl.x==0) return input;
    float4 c=input;float2 p=(pos-surfaceRect.xy)/max(surfaceRect.zw-surfaceRect.xy,0.001);
    float t=surfaceControl.y;float2 physical=floor(pos*surfaceControl.zw);
    if (hasSurface(64)) {
        uint w,h;texture0.GetDimensions(w,h);float2 offset=float2(surfaceDistort.z,0)/float2(w,h);
        c.r=filtered(saturate(uv+offset)).r*modulation.r;c.b=filtered(saturate(uv-offset)).b*modulation.b;
    }
    if (hasSurface(8)) {
        c.rgb=(c.rgb*exp2(surfaceGrade.x)-0.5)*surfaceGrade.y+0.5;
        c.rgb=lerp(dot(c.rgb,float3(0.2126,0.7152,0.0722)),c.rgb,surfaceGrade.z);
    }
    if (hasSurface(16)) c.rgb=lerp(c.rgb,dot(c.rgb,float3(0.2126,0.7152,0.0722)),surfaceGrade.w);
    if (hasSurface(1)) c.rgb+=(hashPixel(floor(pos))-0.5)*surfaceNoise.x;
    if (hasSurface(2)) c.rgb+=(hashPixel(physical+floor(t*24)*float2(17,29))-0.5)*surfaceNoise.y;
    if (hasSurface(4)) {
        uint2 pixel=(uint2)((int2)physical&7);float rank=(BLUE_NOISE[pixel.y*8+pixel.x]+0.5)/64;
        if (surfaceNoise.w<0.5) {
            uint x=pixel.x&3,y=pixel.y&3;
            rank=(((x&1)^(y&1))*8+(y&1)*4+(((x>>1)&1)^((y>>1)&1))*2+((y>>1)&1)+0.5)/16;
        }
        c.rgb+=(rank-0.5)*surfaceNoise.z;
    }
    float spacing=surfacePattern.y;
    if (hasSurface(128)) c.rgb*=1-surfacePattern.x*(0.5+0.5*cos(pos.y*6.2831853/spacing));
    float2 cell=abs(frac(pos/spacing-0.5)-0.5)*spacing;
    float aa=max(abs(ddx(pos.x))+abs(ddy(pos.x)),abs(ddx(pos.y))+abs(ddy(pos.y)));
    if (hasSurface(256)) {float grid_alpha=1-smoothstep(surfacePattern.z*0.5,surfacePattern.z*0.5+aa,min(cell.x,cell.y));c.rgb=lerp(c.rgb,1,grid_alpha*0.3);}
    if (hasSurface(512)) {float a=1-smoothstep(surfacePattern.w,surfacePattern.w+aa,length(cell));c.rgb=lerp(c.rgb,1,a*0.4);}
    if (hasSurface(32)) c.rgb*=1-surfaceLight.x*smoothstep(0.15,0.7,length(p-0.5));
    float sweep=1-smoothstep(0,surfaceLight.z,abs(p.x+p.y*0.25-frac(t*surfaceLight.w)*1.5+0.125));
    if (hasSurface(1024)) c.rgb*=1+sweep*surfaceLight.y;
    if (hasSurface(2048)) c.rgb+=sweep*surfaceLight.y;
    return float4(max(c.rgb*surfaceTint.rgb,0),c.a*surfaceTint.a);
}
