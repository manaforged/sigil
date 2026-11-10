const FILL_KIND_GRADIENT: u32 = 0u;
const FILL_KIND_SLASH: u32 = 1u;
const FILL_KIND_CHECKER: u32 = 2u;
const FILL_KIND_WASH: u32 = 3u;
const FILL_KIND_RADIAL: u32 = 4u;
const FILL_KIND_CONIC: u32 = 5u;

struct Uniforms {
    viewport: vec2<f32>,
    _pad: vec2<f32>,
};

struct GpuStop {
    offset_bits: u32,
    rgba01: u32,
    rgba23: u32,
    _pad: u32,
};

struct StopData {
    offset: f32,
    color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(1) @binding(0) var<storage, read> stops: array<GpuStop>;

struct VertexIn {
    @location(0) unit: vec2<f32>,
};

struct InstanceIn {
    @location(1) rect: vec4<f32>,
    @location(2) line: vec4<f32>,
    @location(3) corner_radii: vec4<f32>,
    @location(4) params: vec4<f32>,
    @location(5) stop_range: vec4<u32>,
    @location(6) stroke_params: vec4<f32>,
    @location(7) stroke_meta: vec4<f32>,
    @location(8) pattern_a: vec4<f32>,
    @location(9) pattern_b: vec4<f32>,
    @location(10) xform_x: vec4<f32>,
    @location(11) xform_y: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) pos_px: vec2<f32>,
    @location(1) local: vec2<f32>,
    @location(2) half_size: vec2<f32>,
    @location(3) corner_radii: vec4<f32>,
    @location(4) opacity: f32,
    @location(5) start: vec2<f32>,
    @location(6) axis: vec2<f32>,
    @location(7) @interpolate(flat) stops_meta: vec4<u32>,
    @location(8) stroke_params: vec4<f32>,
    @location(9) stroke_meta: vec4<f32>,
    @location(10) pattern_a: vec4<f32>,
    @location(11) pattern_b: vec4<f32>,
    @location(12) @interpolate(flat) fill_kind: u32,
    @location(13) dash_origin: vec2<f32>,
};

@vertex
fn vs_main(v: VertexIn, inst: InstanceIn) -> VertexOut {
    var out: VertexOut;
    let pad = aa_pad(inst.xform_x.xy, inst.xform_y.xy);
    let corner = v.unit * (inst.rect.zw + 2.0 * pad) - pad;
    let local_px = inst.rect.xy + corner;
    let pos_px = vec2<f32>(
        dot(inst.xform_x.xy, local_px) + inst.xform_x.z,
        dot(inst.xform_y.xy, local_px) + inst.xform_y.z,
    );
    let ndc = vec2<f32>(
        (pos_px.x / u.viewport.x) * 2.0 - 1.0,
        1.0 - (pos_px.y / u.viewport.y) * 2.0,
    );
    let expanded_half = inst.rect.zw * 0.5;
    let outset = inst.params.y;
    let surface_half = max(expanded_half - vec2<f32>(outset, outset), vec2<f32>(0.0, 0.0));
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.pos_px = local_px;
    out.local = corner - expanded_half;
    out.half_size = surface_half;
    out.corner_radii = inst.corner_radii;
    out.opacity = inst.params.x;
    out.start = inst.line.xy;
    out.axis = inst.line.zw - inst.line.xy;
    out.stops_meta = inst.stop_range;
    out.stroke_params = inst.stroke_params;
    out.stroke_meta = inst.stroke_meta;
    out.dash_origin = inst.params.zw;
    out.pattern_a = inst.pattern_a;
    out.pattern_b = inst.pattern_b;
    out.fill_kind = inst.stop_range.z;
    return out;
}

fn read_stop(base: u32, index: u32) -> StopData {
    let raw = stops[base + index];
    let rg = unpack2x16float(raw.rgba01);
    let ba = unpack2x16float(raw.rgba23);
    var out: StopData;
    out.offset = bitcast<f32>(raw.offset_bits);
    out.color = vec4<f32>(rg.x, rg.y, ba.x, ba.y);
    return out;
}

fn srgb_encode(c: vec3<f32>) -> vec3<f32> {
    let v = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    let high = 1.055 * pow(v, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(high, v * 12.92, v <= vec3<f32>(0.0031308));
}

fn srgb_decode(c: vec3<f32>) -> vec3<f32> {
    let v = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    let high = pow((v + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(high, v / 12.92, v <= vec3<f32>(0.04045));
}

fn gamma_premultiplied(color: vec4<f32>, linear: bool) -> vec4<f32> {
    let rgb = select(color.rgb, srgb_encode(color.rgb), linear);
    return vec4<f32>(rgb * color.a, color.a);
}

fn target_premultiplied(mixed: vec4<f32>, linear: bool) -> vec4<f32> {
    if mixed.a <= 0.0 {
        return vec4<f32>(0.0);
    }
    let rgb = mixed.rgb / mixed.a;
    return vec4<f32>(select(rgb, srgb_decode(rgb), linear) * mixed.a, mixed.a);
}

fn sample_gradient(base: u32, count_in: u32, t: f32, linear: bool) -> vec4<f32> {
    let count = clamp(count_in, 1u, 8u);
    var prev = read_stop(base, 0u);
    if t < prev.offset {
        return premultiply(prev.color);
    }
    for (var i = 1u; i < count; i = i + 1u) {
        let next = read_stop(base, i);
        if t < next.offset {
            let amount = (t - prev.offset) / (next.offset - prev.offset);
            let a = gamma_premultiplied(prev.color, linear);
            let b = gamma_premultiplied(next.color, linear);
            return target_premultiplied(mix(a, b, amount), linear);
        }
        prev = next;
    }
    return premultiply(prev.color);
}

fn gradient_t(kind: u32, p: vec2<f32>, start: vec2<f32>, axis: vec2<f32>, shape: vec4<f32>) -> f32 {
    if kind == FILL_KIND_RADIAL {
        return length((p - start) / max(shape.xy, vec2<f32>(0.0001, 0.0001)));
    }
    if kind == FILL_KIND_CONIC {
        let d = p - start;
        let turn = atan2(d.x, -d.y) / 6.28318530718 - shape.z / 360.0;
        return turn - floor(turn);
    }
    let axis_len = max(dot(axis, axis), 0.0001);
    return dot(p - start, axis) / axis_len;
}

fn repeat_t(t: f32, span: vec4<f32>) -> f32 {
    if span.z < 0.5 {
        return t;
    }
    let width = max(span.y - span.x, 0.0001);
    let r = (t - span.x) / width;
    return span.x + (r - floor(r)) * width;
}

fn premultiply(color: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(color.rgb * color.a, color.a);
}

fn hash_xy(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453123);
}

fn overlay_stroke(fill: vec4<f32>, stroke: vec4<f32>, amount: f32) -> vec4<f32> {
    let stroke_alpha = stroke.a * amount;
    let out_alpha = stroke_alpha + fill.a * (1.0 - stroke_alpha);
    let rgb = stroke.rgb * stroke_alpha + fill.rgb * (1.0 - stroke_alpha);
    return vec4<f32>(rgb, out_alpha);
}

fn slash_color(local: vec2<f32>, geometry: vec2<f32>, color: vec4<f32>) -> vec4<f32> {
    let period = geometry.x;
    let line_width = geometry.y;
    let p = (local.x - local.y) / period;
    let phase = (p - floor(p)) * period;
    let edge_dist = min(phase, period - phase) - line_width * 0.5;
    let aa = fwidth(local.x - local.y) * 0.5;
    let soft = 1.0 - smoothstep(-aa, aa, edge_dist);
    let full = select(soft, 1.0, line_width >= period);
    let mask = select(full, 0.0, line_width <= 0.0);
    return color * mask;
}

fn checker_color(local: vec2<f32>, geometry: vec2<f32>, color_a: vec4<f32>, color_b: vec4<f32>) -> vec4<f32> {
    let cell = geometry.x;
    let cx = floor(local.x / cell);
    let cy = floor(local.y / cell);
    let parity = (cx + cy) - 2.0 * floor((cx + cy) * 0.5);
    return select(color_b, color_a, parity == 0.0);
}

fn wash_hash(x: i32, y: i32, seed: u32) -> u32 {
    var n = (bitcast<u32>(x) * 0x9E3779B1u) ^ (bitcast<u32>(y) * 0x85EBCA77u) ^ (seed * 0xC2B2AE3Du);
    n = n ^ (n >> 16u);
    n = n * 0x7FEB352Du;
    n = n ^ (n >> 15u);
    n = n * 0x846CA68Bu;
    return n ^ (n >> 16u);
}

fn wash_rand(x: i32, y: i32, seed: u32) -> f32 {
    return f32(wash_hash(x, y, seed) >> 8u) / 16777216.0;
}

fn wash_noise(x: f32, y: f32, seed: u32) -> f32 {
    let c = floor(vec2<f32>(x, y));
    let f = vec2<f32>(x, y) - c;
    let s = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let i = vec2<i32>(c);
    let a = wash_rand(i.x, i.y, seed);
    let b = wash_rand(i.x + 1, i.y, seed);
    let d = wash_rand(i.x, i.y + 1, seed);
    let e = wash_rand(i.x + 1, i.y + 1, seed);
    let top = a + (b - a) * s.x;
    let bottom = d + (e - d) * s.x;
    return top + (bottom - top) * s.y;
}

fn wash_fbm(x_in: f32, y_in: f32, octaves: u32, seed: u32) -> f32 {
    var x = x_in;
    var y = y_in;
    var sum = 0.0;
    var weight = 1.0;
    var total = 0.0;
    for (var o = 0u; o < octaves; o = o + 1u) {
        let shift = f32(o);
        sum = sum + wash_noise(x + shift * 17.3, y - shift * 9.1, seed) * weight;
        total = total + weight;
        weight = weight * 0.5;
        let turned = 2.03 * (0.8 * x - 0.6 * y);
        y = 2.03 * (0.6 * x + 0.8 * y);
        x = turned;
    }
    return sum / total;
}

fn wash_ridged(x_in: f32, y_in: f32, seed: u32) -> f32 {
    var x = x_in;
    var y = y_in;
    var sum = 0.0;
    var weight = 1.0;
    var total = 0.0;
    var gain = 1.0;
    for (var o = 0u; o < 4u; o = o + 1u) {
        let shift = f32(o);
        var ridge = 1.0 - abs(2.0 * wash_noise(x + shift * 23.1, y + shift * 5.7, seed) - 1.0);
        ridge = ridge * ridge * gain;
        gain = clamp(ridge * 1.6, 0.0, 1.0);
        sum = sum + ridge * weight;
        total = total + weight;
        weight = weight * 0.55;
        let turned = 2.1 * (0.8 * x - 0.6 * y);
        y = 2.1 * (0.6 * x + 0.8 * y);
        x = turned;
    }
    return sum / total;
}

fn wash_gauss(x: f32) -> f32 {
    return exp(-x * x);
}

fn wash_roll(salt: i32, seed: u32) -> f32 {
    return wash_rand(salt, 0x5eed, seed);
}

fn wash_band(u: f32, v: f32, wx: f32, wy: f32, seed: u32) -> f32 {
    let flip = select(0.0, 3.14159265, wash_roll(2, seed) > 0.5);
    let angle = (wash_roll(1, seed) - 0.5) * 1.8 + flip;
    let sn = sin(angle);
    let cs = cos(angle);
    let along = u * cs + v * sn;
    let curve = (wash_roll(30, seed) - 0.5) * 0.45 * along * along
        + (wash_roll(31, seed) - 0.5) * 0.35 * sin(along * (1.2 + 1.6 * wash_roll(32, seed)));
    let across = v * cs - u * sn - (wash_roll(3, seed) - 0.5) * 0.6 - curve;
    let swell = clamp((wash_fbm(along * 0.55 + 9.0, 2.0, 3u, seed) - 0.5) * 3.0 + 0.5, 0.0, 1.0);
    let width = 0.42 * (0.8 + 0.4 * wash_roll(6, seed)) * (0.7 + 0.65 * swell);
    let ragged = across + 0.1 * (wash_fbm(wx * 3.0 + 70.0, wy * 3.0, 4u, seed) - 0.5);
    let main = wash_gauss(ragged / width);
    let blot = clamp((wash_fbm(wx * 0.9 + 140.0, wy * 0.7 + 3.0, 4u, seed) - 0.56) * 4.0, 0.0, 1.0);
    let cloud = blot * wash_gauss(across / (width * 3.2)) * (1.0 - main) * 0.9;
    let profile = max(main, cloud);
    let clouds = clamp((wash_fbm(wx * 1.7, wy * 2.4, 5u, seed) - 0.3) * 1.8, 0.0, 1.0);
    let lit = profile * (0.38 + 0.72 * clouds);
    let bu = (along - (wash_roll(4, seed) - 0.5) * 1.8) / 0.5;
    let bv = across / 0.24;
    let edge = 0.45 + 1.1 * wash_fbm(wx * 2.2 + 40.0, wy * 2.2, 4u, seed);
    let bulge = exp(-(bu * bu + bv * bv) / edge) * (0.35 + 0.65 * swell);
    let broken = clamp((wash_fbm(wx * 0.9 + 11.0, wy * 2.4, 5u, seed) - 0.34) * 2.4, 0.0, 1.0);
    let breadth = width * 0.22 + 0.025;
    let rift = wash_gauss((across - 0.02) / breadth) * broken * 0.85;
    return clamp((lit * 0.85 + bulge * 0.95) * 1.15 * (1.0 - 0.8 * rift), 0.0, 1.0);
}

fn wash_pool(amount: f32, pools: f32, edges: f32) -> f32 {
    if pools < 1.0 {
        return amount;
    }
    let x = amount * pools;
    let level = floor(x);
    let rest = x - level;
    let settled = (level + smoothstep(0.80, 1.0, rest)) / pools;
    let rim = (1.0 - smoothstep(0.0, 0.16, rest)) * select(0.0, 1.0, level >= 1.0);
    return settled + edges * rim * (0.9 / pools);
}

fn wash_ramp(base: u32, count: u32, t: f32) -> vec3<f32> {
    let pigments = max(count, 2u) - 1u;
    if pigments == 1u {
        return read_stop(base, 1u).color.rgb;
    }
    let at = t * f32(pigments - 1u);
    let index = min(u32(floor(at)), pigments - 2u);
    let a = read_stop(base, 1u + index).color.rgb;
    let b = read_stop(base, 2u + index).color.rgb;
    return mix(a, b, at - f32(index));
}

fn wash_color(p: vec2<f32>, base: u32, count: u32, a: vec4<f32>, b: vec4<f32>, line: vec2<f32>) -> vec4<f32> {
    let scale = line.x;
    let seed = u32(line.y);
    let density = a.x;
    let pools = a.y;
    let edges = a.z;
    let granulation = a.w;
    let grain = b.x;
    let specks = b.y;
    let flags = u32(b.z);
    let band = (flags & 1u) != 0u;
    let dark = (flags & 2u) != 0u;
    let u = p.x / scale;
    let v = p.y / scale;
    let wx = u + 1.1 * (wash_fbm(u * 0.8 + 3.0, v * 0.8 + 7.0, 3u, seed) - 0.5);
    let wy = v + 1.1 * (wash_fbm(u * 0.8 - 5.0, v * 0.8 + 1.0, 3u, seed) - 0.5);
    var body: f32;
    if band {
        let bits = bitcast<u32>(b.w);
        let centre = vec2<f32>(f32(bits & 0xffffu), f32(bits >> 16u)) / scale;
        body = wash_band(u - centre.x, v - centre.y, wx, wy, seed);
    } else {
        body = smoothstep(0.38, 0.78, wash_fbm(wx * 0.9, wy * 0.9, 5u, seed));
    }
    var amount = wash_pool(clamp(body * density, 0.0, 1.0), pools, edges);
    let ridge = wash_ridged(wx * 1.4 + 31.0, wy * 1.4 - 17.0, seed);
    let inside = smoothstep(0.66, 0.74, ridge);
    let fringe = smoothstep(0.60, 0.66, ridge) * (1.0 - inside);
    let wet = smoothstep(0.05, 0.5, amount);
    amount = amount * (1.0 - 0.35 * inside * edges) + fringe * 0.12 * edges * wet;
    let fibre = wash_noise((0.8 * p.x + 0.6 * p.y) / 7.0, (0.8 * p.y - 0.6 * p.x) / 3.0, seed ^ 0x13u);
    let grains = 0.6 * wash_noise(p.x / 1.7, p.y / 1.7, seed ^ 0x51u) + 0.4 * wash_noise(p.x / 5.0, p.y / 5.0, seed ^ 0x77u);
    amount = amount * (1.0 + granulation * (grains - 0.5) * 1.6 * wet);
    amount = clamp(amount * (1.0 + grain * 0.25 * (fibre - 0.5)), 0.0, 1.0);
    let hue = clamp((wash_fbm(wx * 0.6 + 90.0, wy * 0.5, 3u, seed) - 0.3) / 0.4, 0.0, 1.0);
    let pigment = wash_ramp(base, count, hue);
    let paper = read_stop(base, 0u).color;
    var rgb: vec3<f32>;
    if dark {
        rgb = vec3<f32>(1.0) - (vec3<f32>(1.0) - paper.rgb) * (vec3<f32>(1.0) - pigment * amount);
    } else {
        rgb = paper.rgb * (vec3<f32>(1.0 - amount) + pigment * amount);
        let fine = wash_rand(i32(floor(p.x)), i32(floor(p.y)), seed ^ 0x29u);
        rgb = rgb * (1.0 - grain * (0.035 * fine + 0.045 * fibre));
    }
    if specks > 0.0 {
        let cell = floor(p / 4.0);
        let ci = vec2<i32>(cell);
        let sseed = seed ^ 0xabcu;
        if wash_rand(ci.x, ci.y, sseed) < specks * 0.05 * (0.35 + body) {
            let o = cell * 4.0 + 4.0 * vec2<f32>(wash_rand(ci.x, ci.y, sseed ^ 1u), wash_rand(ci.x, ci.y, sseed ^ 2u));
            let radius = 0.6 + 0.7 * wash_rand(ci.x, ci.y, sseed ^ 3u);
            let dot_mask = 1.0 - smoothstep(radius - 0.6, radius + 0.6, distance(p, o));
            if dark {
                let glow = 0.55 + 0.45 * wash_rand(ci.x, ci.y, sseed ^ 4u);
                rgb = rgb + (vec3<f32>(1.0) - rgb) * dot_mask * glow;
            } else {
                let ink = wash_ramp(base, count, 1.0) * 0.45;
                rgb = mix(rgb, ink, dot_mask * 0.55);
            }
        }
    }
    return premultiply(vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), paper.a));
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let d = rounded_rect_sdf(in.local, in.half_size, in.corner_radii);
    let aa = max(fwidth(d) * 0.5, 0.0001);
    let coverage = edge_coverage(d, aa);
    let linear = in.stops_meta.w == 1u;

    var color: vec4<f32>;
    if in.fill_kind == FILL_KIND_SLASH {
        color = slash_color(in.local + in.half_size + in.start + in.axis, in.start, in.pattern_a);
    } else if in.fill_kind == FILL_KIND_WASH {
        color = wash_color(in.local + in.half_size + in.start + in.axis, in.stops_meta.x, in.stops_meta.y, in.pattern_a, in.pattern_b, in.start);
    } else if in.fill_kind == FILL_KIND_CHECKER {
        color = checker_color(in.local + in.half_size + in.start + in.axis, in.start, in.pattern_a, in.pattern_b);
    } else {
        let t = gradient_t(in.fill_kind, in.pos_px, in.start, in.axis, in.pattern_a);
        color = sample_gradient(in.stops_meta.x, in.stops_meta.y, repeat_t(t, in.pattern_b), linear);
    }

    let alpha = color.a * in.opacity * coverage;
    let scale = in.opacity * coverage;
    let dither = hash_xy(in.clip.xy) * (2.0 / 255.0) - (1.0 / 255.0);
    let shaded = color.rgb * scale;
    let stored = select(shaded, srgb_encode(shaded), linear) + vec3<f32>(dither * alpha);
    let rgb = clamp(select(stored, srgb_decode(stored), linear), vec3<f32>(0.0), vec3<f32>(1.0));
    let fill = vec4<f32>(rgb, alpha);
    let amount = stroke_mask(d, in.local + in.dash_origin, in.stroke_meta, aa);
    let stroke = vec4<f32>(in.stroke_params.rgb, in.stroke_params.a * in.opacity);
    return overlay_stroke(fill, stroke, amount);
}
