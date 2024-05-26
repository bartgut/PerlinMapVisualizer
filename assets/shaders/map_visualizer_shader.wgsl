#import noisy_bevy simplex_noise_2d

@group(0) @binding(0)
var texture: texture_storage_2d<rgba8unorm, read_write>;

const CRAWLER_COUNT: u32 = 1300;
const FRAME_SIZE: u32 = 10;

struct Crawler {
    start_pos: vec2<u32>,
    current_radius: u32,
    pixel_color: vec4<f32>,
    map_id: u32
}

struct ParamsUniforms {
    crawlers: array<Crawler, CRAWLER_COUNT>
};


@group(1) @binding(0)
var<uniform> params: ParamsUniforms;

@group(1) @binding(1)
var input_plain_map: texture_storage_2d<rgba8unorm, read>;
@group(1) @binding(2)
var input_traffic_map: texture_storage_2d<rgba8unorm, read>;

fn get_cell(location: vec2<i32>, offset_x: i32, offset_y: i32) -> i32 {
    let value: vec4<f32> = textureLoad(texture, location + vec2<i32>(offset_x, offset_y));
    return i32(value.x);
}

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<u32>(u32(invocation_id.x), u32(invocation_id.y));
    let dim: vec2<u32> = textureDimensions(texture);
    let plain_map_color: vec4<f32> = textureLoad(input_plain_map, location);
    let traffic_map_color: vec4<f32> = textureLoad(input_traffic_map, location);
    let res: vec4<f32> = textureLoad(texture, location);

    var value = res;
    let belongsToFrame = f32(belongs_to_frame(dim, location));
    var is_set = false;
    value = mix(value, vec4<f32>(1.0, 1.0, 1.0, 1.0), belongsToFrame);
    for (var i: u32 = 0; i < CRAWLER_COUNT; i = i + 1) {
        let crawler = params.crawlers[i];
        if (covers_location(crawler, location)) {
            if (crawler.map_id == 0 && is_set == false) { // plain map
               if (all(plain_map_color.rgb == vec3(1.0)) && all(res.rgb == vec3(0.0))) {
                    value = crawler.pixel_color;
               }
            } else { // traffic map
                if (all(traffic_map_color.rgb == vec3(1.0))) {
                    value = crawler.pixel_color;
                    is_set = true;
                    break;
                }
            }
        }
    }

    storageBarrier();
    textureStore(texture, location, value);
}

fn belongs_to_frame(texture_dimention: vec2<u32>, location: vec2<u32>) -> bool {
    return (location.x < FRAME_SIZE) ||
        (location.y < FRAME_SIZE) ||
        (location.x > texture_dimention.x - FRAME_SIZE) ||
        (location.y > texture_dimention.y - FRAME_SIZE);
}

fn covers_location(crawler: Crawler, location: vec2<u32>) -> bool {
    let left = crawler.start_pos.x - crawler.current_radius;
    let right = crawler.start_pos.x + crawler.current_radius;
    let top = crawler.start_pos.y - crawler.current_radius;
    let bottom = crawler.start_pos.y + crawler.current_radius;
    return (location.x >= left) && (location.x <= right) && (location.y >= top) && (location.y <= bottom);
}