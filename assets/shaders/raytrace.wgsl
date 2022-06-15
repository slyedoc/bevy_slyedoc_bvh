struct Settings {
    size: vec2<u32>;
};

struct Camera {
    position: vec3<f32>;
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

[[group(0), binding(0)]]
var output_texture: texture_storage_2d<rgba8unorm, read_write>;

[[group(0), binding(1)]]
var<uniform> camera: Camera;

[[group(0), binding(2)]]
var<uniform> settings: Settings;

[[group(0), binding(3)]]
var background_texture: texture_storage_2d<rgba8unorm, read>;

fn circle(st: vec2<f32>, radius: f32) -> f32 {
    var dist = st - vec2<f32>(0.5);
	return  1.0 - smoothStep(radius - (radius * 0.01),
                         radius + (radius * 0.01),
                         dot(dist, dist) * 4.0);
}

fn get(location: vec2<i32>, offset_x: i32, offset_y: i32) -> i32 {
    let value: vec4<f32> = textureLoad(background_texture, location + vec2<i32>(offset_x, offset_y));
    return i32(value.x);
}


fn trace(ray: Ray) -> vec3<f32>  {
    // sample sky
    var phi = atan2( ray.direction.z, ray.direction.x );
    if (phi <= 0.0) {
        phi = phi + 2.0 * PI;
    }
    let u = u32( 3200.0 * phi * INV2PI - 0.5);
    let v = u32(1600.0 * acos( ray.direction.y ) * INVPI - 0.5);
    let skyIdx = (u + v * u32(3200)) % u32(3200 * 1600);
    //ray: intdx v * 3200  = (u +) % (3200 * 1600)
    //skyPixels[skyIdx * 3], skyPixels[skyIdx * 3 + 1], skyPixels[skyIdx * 3 + 2]);
    return 0.65 * vec3<f32>( 0.0, 0.0, 1.0 );        
}

[[stage(compute), workgroup_size(8, 8, 1)]]
fn init([[builtin(global_invocation_id)]] invocation_id: vec3<u32>, [[builtin(num_workgroups)]] num_workgroups: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let location_f32 = vec2<f32>(f32(invocation_id.x), f32(invocation_id.y));
    let uv = vec2<f32>(location_f32.x / f32(settings.size.x), location_f32.y / f32(settings.size.y));

    //let c = circle(uv, 0.5);

    //let color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    // let randomNumber = randomFloat(invocation_id.y * num_workgroups.x + invocation_id.x);
    // let alive = randomNumber > 0.9;
    // let color = vec4<f32>(f32(alive));

    // plot a pixel into the target array in GPU memory
    //storageBarrier();
    //textureStore(output_texture, location, color);
}

[[stage(compute), workgroup_size(8, 8, 1)]]
fn update([[builtin(global_invocation_id)]] invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let location_f32 = vec2<f32>(f32(invocation_id.x), f32(invocation_id.y));
    let uv = vec2<f32>(location_f32.x / f32(settings.size.x), location_f32.y / f32(settings.size.y));
    // plot a pixel into the target array in GPU memory
    let c = circle(uv, 0.5);
    //let c = textureLoad(background_texture, location);
    let color = vec4<f32>(uv.x + c, uv.y + c, c, 1.0);
    
    storageBarrier();
    textureStore(output_texture, location, color);
}

// fn hash(value: u32) -> u32 {
//     var state = value;
//     state = state ^ 2747636419u;
//     state = state * 2654435769u;
//     state = state ^ state >> 16u;
//     state = state * 2654435769u;
//     state = state ^ state >> 16u;
//     state = state * 2654435769u;
//     return state;
// }

// fn randomFloat(value: u32) -> f32 {
//     return f32(hash(value)) / 4294967295.0;
// }

// fn get(location: vec2<i32>, offset_x: i32, offset_y: i32) -> i32 {
//     let value: vec4<f32> = textureLoad(texture, location + vec2<i32>(offset_x, offset_y));
//     return i32(value.x);
// }

// fn count_alive(location: vec2<i32>) -> i32 {
//     return get(location, -1, -1) +
//            get(location, -1,  0) +
//            get(location, -1,  1) +
//            get(location,  0, -1) +
//            get(location,  0,  1) +
//            get(location,  1, -1) +
//            get(location,  1,  0) +
//            get(location,  1,  1);
// }