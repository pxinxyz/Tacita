fn has_surface(flag: u32) -> bool { return (u32(effects.surface_control.x) & flag) != 0u; }
fn hash_pixel(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p,vec2<f32>(127.1,311.7))) * 43758.5453);
}
fn straight(c: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(c.rgb / max(c.a,0.000001),c.a);
}
fn weighted(c: vec4<f32>) -> vec4<f32> { return vec4<f32>(c.rgb*c.a,c.a); }
fn image_linear(uv: vec2<f32>) -> vec4<f32> {
    let size=vec2<i32>(textureDimensions(atlas_tex));
    let p=clamp(uv,vec2<f32>(0),vec2<f32>(1))*vec2<f32>(size)-0.5;
    let base=vec2<i32>(floor(p)); let f=fract(p);
    let a=gui_input_color(textureLoad(atlas_tex,clamp(base,vec2<i32>(0),size-1),0),1u);
    let b=gui_input_color(textureLoad(atlas_tex,clamp(base+vec2<i32>(1,0),vec2<i32>(0),size-1),0),1u);
    let c=gui_input_color(textureLoad(atlas_tex,clamp(base+vec2<i32>(0,1),vec2<i32>(0),size-1),0),1u);
    let d=gui_input_color(textureLoad(atlas_tex,clamp(base+vec2<i32>(1,1),vec2<i32>(0),size-1),0),1u);
    return straight(mix(mix(weighted(a),weighted(b),f.x),mix(weighted(c),weighted(d),f.x),f.y));
}
fn secondary_linear(uv:vec2<f32>)->vec4<f32> {
    let size=vec2<i32>(textureDimensions(mask3));
    let p=clamp(uv,vec2<f32>(0),vec2<f32>(1))*vec2<f32>(size)-0.5;
    let base=vec2<i32>(floor(p));let f=fract(p);
    let a=gui_input_color(textureLoad(mask3,clamp(base,vec2<i32>(0),size-1),0),4u);
    let b=gui_input_color(textureLoad(mask3,clamp(base+vec2<i32>(1,0),vec2<i32>(0),size-1),0),4u);
    let c=gui_input_color(textureLoad(mask3,clamp(base+vec2<i32>(0,1),vec2<i32>(0),size-1),0),4u);
    let d=gui_input_color(textureLoad(mask3,clamp(base+vec2<i32>(1,1),vec2<i32>(0),size-1),0),4u);
    return straight(mix(mix(weighted(a),weighted(b),f.x),mix(weighted(c),weighted(d),f.x),f.y));
}
fn filtered(uv:vec2<f32>)->vec4<f32> {
    let kind=u32(effects.filter_control.x);
    if (kind==0u) {
        if (has_surface(64u) || has_surface(4096u) || has_surface(8192u)) {return image_linear(uv);}
        return gui_input_color(textureSampleLevel(atlas_tex,atlas_smp,uv,0.0),1u);
    }
    let texel=select(1.0/vec2<f32>(textureDimensions(atlas_tex)),effects.filter_values.yz,all(effects.filter_values.yz>vec2<f32>(0)));
    let radius=effects.filter_control.z;
    if (kind==1u || kind==2u) {
        let axis=select(vec2<f32>(0,1),vec2<f32>(1,0),kind==1u);
        var sum=vec4<f32>(0);var total=0.0;
        for (var i=-8;i<=8;i+=1) {
            let weight=exp(-0.5*(f32(i)/2.6666667)*(f32(i)/2.6666667));
            sum+=weighted(image_linear(uv+axis*texel*f32(i)*radius/8.0))*weight;total+=weight;
        }
        return straight(sum/total);
    }
    if (kind==3u) {
        let d=texel*max(radius,0.5);
        return straight((weighted(image_linear(uv))*4.0+
            weighted(image_linear(uv+vec2<f32>(-d.x,-d.y)))+
            weighted(image_linear(uv+vec2<f32>(d.x,-d.y)))+
            weighted(image_linear(uv+vec2<f32>(-d.x,d.y)))+
            weighted(image_linear(uv+d)))/8.0);
    }
    if (kind==4u) {
        let d=texel*max(radius,0.5);
        var sum=weighted(image_linear(uv+vec2<f32>(-d.x*2.0,0)))+
            weighted(image_linear(uv+vec2<f32>(d.x*2.0,0)))+
            weighted(image_linear(uv+vec2<f32>(0,-d.y*2.0)))+
            weighted(image_linear(uv+vec2<f32>(0,d.y*2.0)));
        sum+=(weighted(image_linear(uv-d))+weighted(image_linear(uv+d))+
            weighted(image_linear(uv+vec2<f32>(-d.x,d.y)))+
            weighted(image_linear(uv+vec2<f32>(d.x,-d.y))))*2.0;
        return straight(sum/12.0);
    }
    let c=image_linear(uv);
    if (kind==5u) {
        let peak=max(c.r,max(c.g,c.b));
        return vec4<f32>(c.rgb*max(peak-effects.filter_control.w,0.0)/max(peak,0.000001),c.a);
    }
    if (kind==6u) {
        let original=secondary_linear(uv);
        let glow=c.rgb*c.a*effects.filter_values.x;
        let a=max(original.a,clamp(dot(glow,vec3<f32>(0.2126,0.7152,0.0722)),0.0,1.0));
        return vec4<f32>((original.rgb*original.a+glow)/max(a,0.000001),a);
    }
    return c;
}
fn warped(uv:vec2<f32>,pos:vec2<f32>)->vec2<f32> {
    if (!has_surface(4096u) && !has_surface(8192u)) {return uv;}
    let p=(pos-effects.surface_rect.xy)/max(effects.surface_rect.zw-effects.surface_rect.xy,vec2<f32>(0.001));
    let t=effects.surface_control.y;
    let texel=1.0/vec2<f32>(textureDimensions(atlas_tex));
    var offset=vec2<f32>(0);
    if (has_surface(4096u)) {offset+=sin(vec2<f32>(p.y,p.x)*effects.surface_distort.y*6.2831853+t)*effects.surface_distort.x;}
    if (has_surface(8192u)) {offset+=(p-0.5)*(dot(p-0.5,p-0.5)*4.0)*effects.surface_distort.w;}
    return clamp(uv+offset*texel,vec2<f32>(0),vec2<f32>(1));
}
fn surface_color(input:vec4<f32>,uv:vec2<f32>,pos:vec2<f32>,modulation:vec4<f32>)->vec4<f32> {
    if (effects.surface_control.x==0.0) {return input;}
    var c=input;
    let p=(pos-effects.surface_rect.xy)/max(effects.surface_rect.zw-effects.surface_rect.xy,vec2<f32>(0.001));
    let t=effects.surface_control.y;
    let physical=floor(pos*effects.surface_control.zw);
    if (has_surface(64u)) {
        let offset=vec2<f32>(effects.surface_distort.z,0)/vec2<f32>(textureDimensions(atlas_tex));
        c.r=filtered(clamp(uv+offset,vec2<f32>(0),vec2<f32>(1))).r*modulation.r;
        c.b=filtered(clamp(uv-offset,vec2<f32>(0),vec2<f32>(1))).b*modulation.b;
    }
    if (has_surface(8u)) {
        c=vec4<f32>((c.rgb*exp2(effects.surface_grade.x)-0.5)*effects.surface_grade.y+0.5,c.a);
        let l=dot(c.rgb,vec3<f32>(0.2126,0.7152,0.0722));
        c=vec4<f32>(mix(vec3<f32>(l),c.rgb,effects.surface_grade.z),c.a);
    }
    if (has_surface(16u)) {c=vec4<f32>(mix(c.rgb,vec3<f32>(dot(c.rgb,vec3<f32>(0.2126,0.7152,0.0722))),effects.surface_grade.w),c.a);}
    if (has_surface(1u)) {c=vec4<f32>(c.rgb+(hash_pixel(floor(pos))-0.5)*effects.surface_noise.x,c.a);}
    if (has_surface(2u)) {c=vec4<f32>(c.rgb+(hash_pixel(physical+floor(t*24.0)*vec2<f32>(17,29))-0.5)*effects.surface_noise.y,c.a);}
    if (has_surface(4u)) {
        let pixel=vec2<u32>(vec2<i32>(physical)&vec2<i32>(7));
        var rank=(f32(BLUE_NOISE[pixel.y*8u+pixel.x])+0.5)/64.0;
        if (effects.surface_noise.w<0.5) {
            let x=pixel.x&3u;let y=pixel.y&3u;
            rank=(f32(((x&1u)^(y&1u))*8u+(y&1u)*4u+(((x>>1u)&1u)^((y>>1u)&1u))*2u+((y>>1u)&1u))+0.5)/16.0;
        }
        c=vec4<f32>(c.rgb+(rank-0.5)*effects.surface_noise.z,c.a);
    }
    let spacing=effects.surface_pattern.y;
    if (has_surface(128u)) {c=vec4<f32>(c.rgb*(1.0-effects.surface_pattern.x*(0.5+0.5*cos(pos.y*6.2831853/spacing))),c.a);}
    let cell=abs(fract(pos/spacing-0.5)-0.5)*spacing;
    if (has_surface(256u)) {let grid_alpha=1.0-smoothstep(effects.surface_pattern.z*0.5,effects.surface_pattern.z*0.5+max(fwidth(pos.x),fwidth(pos.y)),min(cell.x,cell.y));c=vec4<f32>(mix(c.rgb,vec3<f32>(1),grid_alpha*0.3),c.a);}
    if (has_surface(512u)) {let dot_alpha=1.0-smoothstep(effects.surface_pattern.w,effects.surface_pattern.w+max(fwidth(pos.x),fwidth(pos.y)),length(cell));c=vec4<f32>(mix(c.rgb,vec3<f32>(1),dot_alpha*0.4),c.a);}
    if (has_surface(32u)) {c=vec4<f32>(c.rgb*(1.0-effects.surface_light.x*smoothstep(0.15,0.7,length(p-0.5))),c.a);}
    let sweep=1.0-smoothstep(0.0,effects.surface_light.z,abs(p.x+p.y*0.25-fract(t*effects.surface_light.w)*1.5+0.125));
    if (has_surface(1024u)) {c=vec4<f32>(c.rgb*(1.0+sweep*effects.surface_light.y),c.a);}
    if (has_surface(2048u)) {c=vec4<f32>(c.rgb+sweep*effects.surface_light.y,c.a);}
    return vec4<f32>(max(c.rgb*effects.surface_tint.rgb,vec3<f32>(0)),c.a*effects.surface_tint.a);
}
