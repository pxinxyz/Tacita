const GUI_MANAGED_SRGB:bool=false;
fn gui_linear_color(color:vec4<f32>)->vec4<f32>{
 if (!GUI_MANAGED_SRGB){return color;}
 return vec4<f32>(select(pow((color.rgb+vec3<f32>(0.055))/1.055,vec3<f32>(2.4)),color.rgb/12.92,color.rgb<=vec3<f32>(0.04045)),color.a);
}
struct Uniforms { proj: mat4x4<f32> };
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var atlas_tex: texture_2d<f32>;
@group(0) @binding(2) var atlas_smp: sampler;
struct Stop { position: vec4<f32>, color: vec4<f32> };
struct Mask { rect: vec4<f32>, params: vec4<f32>, kind: vec4<f32> };
struct Effects {
    rect: vec4<f32>, geometry: vec4<f32>, parameters: vec4<f32>, mask_count: vec4<f32>,
    stops: array<Stop,16>, masks: array<Mask,4>,
    sdf_rect: vec4<f32>, sdf_radii: vec4<f32>, sdf_parameters: vec4<f32>, sdf_light: vec4<f32>,
    surface_rect:vec4<f32>, surface_control:vec4<f32>, surface_noise:vec4<f32>, surface_grade:vec4<f32>,
    surface_tint:vec4<f32>, surface_pattern:vec4<f32>, surface_light:vec4<f32>, surface_distort:vec4<f32>,
    filter_control:vec4<f32>, filter_values:vec4<f32>, filter_extra:array<vec4<f32>,2>,
};
@group(1) @binding(0) var<uniform> effects: Effects;
@group(1) @binding(1) var mask0: texture_2d<f32>;
@group(1) @binding(2) var mask1: texture_2d<f32>;
@group(1) @binding(3) var mask2: texture_2d<f32>;
@group(1) @binding(4) var mask3: texture_2d<f32>;
struct VsIn { @location(0) pos: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) col: vec4<f32> };
struct VsOut {
    @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32>,
    @location(1) col: vec4<f32>, @location(2) logical: vec2<f32>,
};
@vertex fn vs_main(input: VsIn) -> VsOut {
    var output: VsOut;
    output.pos = uniforms.proj * vec4<f32>(input.pos,0,1);
    output.uv = input.uv; output.col = gui_linear_color(input.col); output.logical = input.pos;
    return output;
}
struct QuadIn {
    @location(0) p0:vec2<f32>, @location(1) uv0:vec2<f32>, @location(2) c0:vec4<f32>,
    @location(3) p1:vec2<f32>, @location(4) uv1:vec2<f32>, @location(5) c1:vec4<f32>,
    @location(6) p2:vec2<f32>, @location(7) uv2:vec2<f32>, @location(8) c2:vec4<f32>,
    @location(9) p3:vec2<f32>, @location(10) uv3:vec2<f32>, @location(11) c3:vec4<f32>,
};
@vertex fn vs_quad(input:QuadIn,@builtin(vertex_index) vertex:u32)->VsOut {
    var position=input.p0; var uv=input.uv0; var color=input.c0;
    switch vertex {
        case 1u: {position=input.p1; uv=input.uv1; color=input.c1;}
        case 2u, 4u: {position=input.p2; uv=input.uv2; color=input.c2;}
        case 5u: {position=input.p3; uv=input.uv3; color=input.c3;}
        default: {}
    }
    var output:VsOut;
    output.pos=uniforms.proj*vec4<f32>(position,0,1);
    output.logical=position; output.uv=uv;
    output.col=gui_linear_color(color);
    return output;
}
fn gui_input_color(color:vec4<f32>,bit:u32)->vec4<f32> {
    if ((u32(effects.mask_count.y)&bit)==0u) {return color;}
    return vec4<f32>(select(vec3<f32>(0),color.rgb/max(color.a,0.000001),color.a>0.0),color.a);
}
fn spread(value: f32, mode: f32) -> f32 {
    if (mode < 0.5) { return clamp(value,0.0,1.0); }
    if (mode < 1.5) { return fract(value); }
    let p = value-floor(value/2.0)*2.0;
    return select(2.0-p,p,p<=1.0);
}
fn paint(pos: vec2<f32>) -> vec4<f32> {
    if (effects.parameters.x < 0.5) { return vec4<f32>(1); }
    let p = (pos-effects.rect.xy)/(effects.rect.zw-effects.rect.xy)-effects.geometry.xy;
    let v = effects.geometry.zw;
    var t: f32;
    if (effects.parameters.x < 1.5) { t = dot(p,v)/dot(v,v); }
    else if (effects.parameters.x < 2.5) { t = length(p/v); }
    else { t = fract((atan2(p.y,p.x)-atan2(v.y,v.x))/6.28318530718+1.0); }
    t = spread(t+effects.parameters.z,effects.parameters.y);
    let count = min(u32(effects.parameters.w),16u);
    for (var i=1u; i<count; i+=1u) {
        if (t <= effects.stops[i].position.x) {
            let f = clamp((t-effects.stops[i-1u].position.x)/(effects.stops[i].position.x-effects.stops[i-1u].position.x),0.0,1.0);
            return mix(gui_linear_color(effects.stops[i-1u].color),gui_linear_color(effects.stops[i].color),f);
        }
    }
    return gui_linear_color(effects.stops[max(count,1u)-1u].color);
}
fn sample_mask(index: u32, uv: vec2<f32>) -> f32 {
    if (index == 0u) { return textureSampleLevel(mask0,atlas_smp,uv,0).a; }
    if (index == 1u) { return textureSampleLevel(mask1,atlas_smp,uv,0).a; }
    if (index == 2u) { return textureSampleLevel(mask2,atlas_smp,uv,0).a; }
    return textureSampleLevel(mask3,atlas_smp,uv,0).a;
}

fn shape_distance(pos: vec2<f32>) -> vec3<f32> {
    let p = pos-(effects.sdf_rect.xy+effects.sdf_rect.zw)*0.5;
    let top = select(effects.sdf_radii.y,effects.sdf_radii.x,p.x<0.0);
    let bottom = select(effects.sdf_radii.z,effects.sdf_radii.w,p.x<0.0);
    let radius = select(bottom,top,p.y<0.0);
    let q = abs(p)-(effects.sdf_rect.zw-effects.sdf_rect.xy)*0.5+radius;
    let outside = max(q,vec2<f32>(0));
    let len = length(outside);
    let axis = select(vec2<f32>(0,1),vec2<f32>(1,0),q.x>q.y);
    let normal = select(axis,outside/max(len,0.000001),len>0.000001)
        * select(vec2<f32>(1),vec2<f32>(-1),p<vec2<f32>(0));
    return vec3<f32>(len+min(max(q.x,q.y),0.0)-radius,normal);
}
fn covered(distance: f32, aa: f32) -> f32 {
    return select(select(0.0,1.0,distance<=0.0),clamp(0.5-distance/aa,0.0,1.0),effects.sdf_light.w>0.5);
}
fn shape_coverage(pos: vec2<f32>) -> f32 {
    let mode = effects.sdf_parameters.x;
    if (mode < 0.5) { return 1.0; }
    let distance_normal = shape_distance(pos);
    let d = distance_normal.x;
    let aa = max(length(vec2<f32>(dpdx(d),dpdy(d))),0.000001);
    if (mode < 1.5) { return covered(d,aa); }
    if (mode < 2.5) {
        let width = effects.sdf_parameters.y;
        let high = select(select(width,width*0.5,effects.sdf_parameters.z<1.5),0.0,effects.sdf_parameters.z<0.5);
        return max(covered(d-high,aa)-covered(d-high+width,aa),0.0);
    }
    if (mode < 4.5) {
        let reach = max(effects.sdf_parameters.w,0.000001);
        let distance = select(abs(d),max(d,0.0),mode<3.5);
        return clamp((exp(-distance/reach)-exp(-6.0))/(1.0-exp(-6.0)),0.0,1.0);
    }
    let band = max(covered(d,aa)-covered(d+effects.sdf_parameters.y,aa),0.0);
    return band*pow(clamp(dot(distance_normal.yz,effects.sdf_light.xy),0.0,1.0),effects.sdf_light.z);
}
fn reveal(m: Mask, pos: vec2<f32>) -> f32 {
    let p = (pos-m.rect.xy)/(m.rect.zw-m.rect.xy);
    var t = p.x;
    if (m.kind.x < 3.5) { t = p.x; }
    else if (m.kind.x < 4.5) { t = 1.0-p.x; }
    else if (m.kind.x < 5.5) { t = p.y; }
    else if (m.kind.x < 6.5) { t = 1.0-p.y; }
    else { t = length((p-0.5)*2.0)/1.41421356237; }
    let progress = m.params.x;
    var a: f32;
    if (progress <= 0.0) { a = 0.0; }
    else if (progress >= 1.0) { a = 1.0; }
    else if (m.params.y <= 0.0) { a = select(0.0,1.0,t<=progress); }
    else {
        let edge = mix(-m.params.y*0.5,1.0+m.params.y*0.5,progress);
        a = 1.0-smoothstep(edge-m.params.y*0.5,edge+m.params.y*0.5,t);
    }
    a = select(a,1.0-a,m.params.z>0.5);
    return select(0.0,a,all(p>=vec2<f32>(0)) && all(p<=vec2<f32>(1)));
}

@fragment fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    let uv=warped(input.uv,input.logical);
    let painting=paint(input.logical);
    var color = surface_color(input.col*filtered(uv)*painting,uv,input.logical,input.col*painting);
    color.a *= shape_coverage(input.logical);
    let count = min(u32(effects.mask_count.x),4u);
    for (var i=0u; i<count; i+=1u) {
        let m = effects.masks[i];
        if (m.kind.x < 1.5) {
            let p = (input.logical-m.rect.xy)/(m.rect.zw-m.rect.xy);
            let inside = select(0.0,1.0,all(p>=vec2<f32>(0)) && all(p<=vec2<f32>(1)));
            color.a *= inside * sample_mask(i,mix(m.params.xy,m.params.zw,clamp(p,vec2<f32>(0),vec2<f32>(1))));
        } else if (m.kind.x < 2.5) {
            let d = m.rect.zw-m.rect.xy;
            let t = clamp(dot(input.logical-m.rect.xy,d)/dot(d,d),0.0,1.0);
            color.a *= mix(m.params.x,m.params.y,t);
        } else {
            color.a *= reveal(m,input.logical);
        }
    }
    if (effects.mask_count.z>0.5 && shape_coverage(input.logical)<0.5) {discard;}
    if ((u32(effects.mask_count.y)&2u)!=0u) {color=vec4<f32>(color.rgb*color.a,color.a);}
    return color;
}
