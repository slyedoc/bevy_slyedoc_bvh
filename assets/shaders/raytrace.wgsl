#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_bindings
#import bevy_pbr::mesh_functions

struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] world_normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
};

struct Camera {
    size: vec2<u32>;
    origin: vec3<f32>;
    p0: vec3<f32>;
    p1: vec3<f32>;
    p2: vec3<f32>;
};

struct Intersection {
    t: f32;                    // intersection distance along ray
    u: f32;
    v: f32;	                // barycentric coordinates of the intersection
    inst_prim: u32;              // instance index (12 bit) and primitive index (20 bit)
};
 
struct Ray {
    origin: vec3<f32>;
    direction: vec3<f32>;
    inv_direction: vec3<f32>;
    hit: Intersection;  
};

let PI: f32 = 3.141592653589793;
let INVPI: f32 =  0.318181818;
let INV2PI: f32 = 0.159154943;


[[group(1), binding(0)]]
var<uniform> camera: Camera;

[[group(1), binding(1)]]
var background_texture: texture_cube<f32>;

[[group(1), binding(2)]]
var background_sampler: sampler;
 //<rgba8unorm, read>;

fn circle(st: vec2<f32>, radius: f32) -> f32 {
    var dist = st - vec2<f32>(0.5);
	return  1.0 - smoothStep(radius - (radius * 0.01),
                         radius + (radius * 0.01),
                         dot(dist, dist) * 4.0);
}


fn trace(ray: Ray) -> vec4<f32>  {
    // sample sky
    // var phi = atan2( ray.direction.z, ray.direction.x );
    // if (phi <= 0.0) {
    //     phi = phi + 2.0 * PI;
    // }
    // let u = u32( 3200.0 * phi * INV2PI - 0.5);
    // let v = u32(1600.0 * acos( ray.direction.y ) * INVPI - 0.5);
    // let skyIdx = (u + v * u32(3200)) % u32(3200 * 1600);
    //ray: intdx v * 3200  = (u +) % (3200 * 1600)
    //skyPixels[skyIdx * 3], skyPixels[skyIdx * 3 + 1], skyPixels[skyIdx * 3 + 2]);
    let color = textureSample(background_texture, background_sampler, ray.direction );
    return 0.65 * color;        
}

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    var model = mesh.model;
    out.world_position = mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
    out.world_normal = mesh_normal_local_to_world(vertex.normal);
    out.uv = vertex.uv;

    out.clip_position = mesh_position_world_to_clip(out.world_position);
    return out;
}

struct FragmentInput {
    [[builtin(front_facing)]] is_front: bool;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] world_normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
};


[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {

    // converting uv to pixel coordinates
    let x =  in.uv.x * f32(camera.size.x);
    let y =  in.uv.y * f32(camera.size.y);
    // create a primary ray for the pixel
    var ray : Ray;
    ray.origin = camera.origin;

    let pixel_pos = ray.origin + camera.p0 +
        (camera.p1 - camera.p0) * in.uv.x +
        (camera.p2 - camera.p0) * in.uv.y;
    ray.direction = normalize( pixel_pos - ray.origin );
    ray.inv_direction = 1.0 / ray.direction;
    ray.hit.t = 1e30; // 1e30f denotes 'no hit'

    // trace the primary ray
    var hit = trace(ray);
    // var ray: Ray = Ray {
    //     origin: camera.position,
    //     direction: normalize(in.world_position.xyz - camera.position),
    //     inv_direction: vec3<f32>(1.0 / in.world_position.xyz),
    // };

    return vec4<f32>( hit );
}

