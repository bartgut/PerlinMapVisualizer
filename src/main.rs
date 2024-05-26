use std::borrow::Cow;
use std::string::ToString;
use bevy::prelude::*;
use bevy::time::TimerMode;
use bevy::ecs::schedule::OnEnter;
use bevy::app::{App, Startup};
use bevy::DefaultPlugins;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::math::{vec2, Vec3};
use bevy::prelude::{Asset, AssetServer, Commands, Component, Image, in_state, IntoSystemConfigs, NextState, Query, Res, ResMut, Resource, States, Timer, TypePath, Update, UVec2};
use bevy::reflect::Map;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType, TextureUsages};
use bevy::render::texture::ImageLoaderSettings;
use bevy_pixel_buffer::builder::{PixelBufferBuilder};
use bevy_pixel_buffer::frame::GetFrame;
use bevy_pixel_buffer::pixel::Pixel;
use bevy_pixel_buffer::prelude::{ComputeShader, ComputeShaderPlugin, PixelBufferPlugin, PixelBufferSize};
use bevy::time::Time;
use bevy_pixel_buffer::pixel_buffer::PixelBuffer;
use bevy_pixel_buffer::query::QueryPixelBuffer;
use noisy_bevy::simplex_noise_2d;
use noisy_bevy::NoisyShaderPlugin;
use rand::Rng;

const PERLIN_NOISE_CONST: f32 = 0.002;
const IMAGE_WIDTH: u32 = 1104;
const IMAGE_HEIGHT: u32 = 872;
const TIMER_PACE: f32 = 0.016;
const RADIUS_UPDATE: u32 = 2;
const CRAWLERS_COUNT: usize = 1300;
const PLAIN_LONDON_MAP_NAME: &str = "london_yellow.jpg";
const TRAFFIC_LONDON_MAP_NAME: &str = "london_t.jpg";
const PLAIN_ISLAND_MAP_NAME: &str = "island_p.jpg";
const TRAFFIC_ISLAND_MAP_NAME: &str = "island_t.jpg";
#[derive(Resource)]
struct FrameCounter {
    count: u32,
}
#[derive(Resource)]
struct Clock(Timer);

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum AppState {
    #[default]
    Preload,
    Preprocessing,
    Running
}

#[derive(Component, Debug)]
struct Crawler {
    id: u32,
    start_pos: UVec2,
    pixel_radius: u32,
    current_radius: u32,
    pixel_color: Vec4,
    map_id: u32
}

#[derive(Component, ShaderType, Clone, Copy, Debug)]
struct CrawlerGPU {
    start_pos: UVec2,
    current_radius: u32,
    pixel_color: Vec4,
    map_id: u32
}

impl Default for Crawler {
    fn default() -> Self {
        Self {
            id: 0,
            start_pos:  UVec2::new(700, 500),
            pixel_radius: 2,
            current_radius: 0,
            pixel_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            map_id: 0
        }
    }
}

impl Default for CrawlerGPU {
    fn default() -> Self {
        Self {
            start_pos:  UVec2::new(700, 500),
            current_radius: 0,
            pixel_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            map_id: 0
        }
    }
}

impl Crawler {
    fn create_as_swarm_part(element_id: u32, frame_counter: u32, pixel_color: Vec4, map_id: u32) -> Self {
        Self {
            id: element_id,
            start_pos: calculate_start_pos(element_id, frame_counter),
            pixel_radius: 2,
            current_radius: 0,
            pixel_color,
            map_id
        }
    }
    fn to_gpu(&self) -> CrawlerGPU {
        CrawlerGPU {
            start_pos: self.start_pos,
            current_radius: self.current_radius,
            pixel_color: self.pixel_color,
            map_id: self.map_id
        }
    }
}

fn calculate_start_pos(element_id: u32, frame_counter: u32) -> UVec2 {
    let x = vec2(PERLIN_NOISE_CONST * (frame_counter as f32) + 1000.0 * (element_id as f32), 0.0);
    let y = vec2(PERLIN_NOISE_CONST * (frame_counter as f32) + 2000.0 * (element_id as f32), 0.0);
    let res =  UVec2::new(((simplex_noise_2d(x) + 1.0)/2.0 * (IMAGE_WIDTH as f32)).round() as u32, ((simplex_noise_2d(y) + 1.0)/2.0 * (IMAGE_HEIGHT as f32)).round() as u32);
    return res;
}

#[derive(Resource)]
struct Maps {
    plain: Handle<Image>,
    traffic: Handle<Image>,
}

fn main() {

    App::new()
        .add_plugins((
            DefaultPlugins,
            PixelBufferPlugin,
            ComputeShaderPlugin::<MapVisualizerShader>::default(),
            NoisyShaderPlugin,
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default()
        ))
        .insert_resource(FrameCounter { count: 1 })
        .init_state::<AppState>()
        .add_systems(Startup, image_preload)
        .add_systems(Update, image_preprocess.run_if(in_state(AppState::Preprocessing)))
        .add_systems(OnEnter(AppState::Running), spawn_crawlers)
        .add_systems(OnEnter(AppState::Running), setup)
        .add_systems(Update, update_crawlers.run_if(in_state(AppState::Running)))
        .add_systems(Update, image_rotate.run_if(in_state(AppState::Running)))
        .add_systems(Update, user_input.run_if(in_state(AppState::Running)))
        .add_systems(Update, param_update.run_if(in_state(AppState::Running)))
        .run()
}


fn spawn_crawlers(mut commands: Commands, frame_counter: Res<FrameCounter>) {
    for i in 0..CRAWLERS_COUNT {
        if i % 5 == 0 {
            let color = Vec4::new(1.0, 0.0, 0.0, 1.0);
            commands.spawn(Crawler::create_as_swarm_part(i.try_into().unwrap(), frame_counter.count, color, 1));
        } else {
            let color = Vec4::new(1.0,1.0,1.0,0.6);
            commands.spawn(Crawler::create_as_swarm_part(i.try_into().unwrap(), frame_counter.count, color, 0));
        }
    }
    commands.insert_resource(Clock(Timer::from_seconds(TIMER_PACE, TimerMode::Repeating)));
}

fn image_rotate(mut query: Query<&mut Transform, With<PixelBuffer>>) {
    for mut transform in query.iter_mut() {
        transform.rotate(Quat::from_rotation_z(0.0001));
    }
}

fn user_input(keyboard_input: Res<ButtonInput<KeyCode>>,
              mut pb: QueryPixelBuffer,
              mut query: Query<&mut Transform, With<PixelBuffer>>,
              mut maps: ResMut<Maps>,
              asset_server: Res<AssetServer>) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        std::process::exit(0);
    }
    if keyboard_input.just_pressed(KeyCode::KeyR) {
        pb.frame().per_pixel(|_,_| Pixel::BLACK);
        for mut transform in query.iter_mut() {
            transform.rotation = Quat::from_rotation_z(0.0);
        }
    }
    if keyboard_input.just_pressed(KeyCode::Digit1) {

    }
}

fn update_crawlers(mut crawlers: Query<&mut Crawler>, time: Res<Time>, mut clock: ResMut<Clock>, mut frame_counter: ResMut<FrameCounter> ) {
    let mut frame_update: bool = false;
    if clock.0.tick(time.delta()).just_finished() {
        for mut crawler in crawlers.iter_mut() {
            crawler.current_radius += RADIUS_UPDATE;
            crawler.pixel_color = crawler.pixel_color.xyz().extend(crawler.pixel_color.w + 0.001);
            if crawler.current_radius > crawler.pixel_radius {
                crawler.current_radius = 0;
                if frame_update == false {
                    frame_counter.count += 1;
                    frame_update = true;
                }
                crawler.start_pos = calculate_start_pos(crawler.id, frame_counter.count);
            }
        }
    }
}

fn image_preload(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<AppState>>
) {
    let settings = move |s: &mut ImageLoaderSettings| {
        s.is_srgb = false
    };

    let map_plain: Handle<Image> = asset_server.load_with_settings(PLAIN_ISLAND_MAP_NAME, settings);
    let map_traffic: Handle<Image> = asset_server.load_with_settings(TRAFFIC_ISLAND_MAP_NAME, settings);
    commands.insert_resource(Maps {
        plain: map_plain,
        traffic: map_traffic,
    });
    next_state.set(AppState::Preprocessing);
}

fn image_preprocess(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    maps: Res<Maps>,
    mut next_state: ResMut<NextState<AppState>>,
    mut plain_loaded: Local<bool>,
    mut traffic_loaded: Local<bool>
) {
    if *plain_loaded == false {
        if let Some(loaded_plain_map) = images.get_mut(&maps.plain) {
            let road_color = Vec3::new(249.0, 255.0, 6.0);
            //let road_color = Vec3::new(216.0, 224.0, 231.0);
            loaded_plain_map.frame().per_pixel(|location, pixel| {
                if Vec3::new(pixel.r.into(), pixel.g.into(), pixel.b.into()).distance(road_color) < 150.0 {
                    return Pixel::WHITE;
                }
                return Pixel::BLACK;
            });
            loaded_plain_map.texture_descriptor.usage = TextureUsages::STORAGE_BINDING;
            *plain_loaded = true;
        }
    }

    if *traffic_loaded == false {
        let traffic_green_color = Vec3::new(249.0, 255.0, 6.0);
        //let traffic_green_color = Vec3::new(20.0, 224.0, 151.0);
        if let Some(loaded_traffic_map) = images.get_mut(&maps.traffic) {
            loaded_traffic_map.frame().per_pixel(|location, pixel| {
                if Vec3::new(pixel.r.into(), pixel.g.into(), pixel.b.into()).distance(traffic_green_color) < 100.0 {
                    return Pixel::WHITE;
                }
                return Pixel::BLACK;
            });
            loaded_traffic_map.texture_descriptor.usage = TextureUsages::STORAGE_BINDING;
            *traffic_loaded = true;
        }
    }

    if *plain_loaded && *traffic_loaded {
        next_state.set(AppState::Running);
    }
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut cs: ResMut<Assets<MapVisualizerShader>>,
    maps: Res<Maps>,
) {
    let size = PixelBufferSize {
        size: UVec2::new(IMAGE_WIDTH, IMAGE_HEIGHT),
        pixel_size: UVec2::new(1, 1)
    };

    PixelBufferBuilder::new()
        .with_size(size)
        .spawn(&mut commands, &mut images)
        .edit_frame(|frame| {
            frame.per_pixel(|location, pixel| {
                return Pixel::BLACK;
            })
        })
        .entity()
        .insert(cs.add(MapVisualizerShader {
            input_plain_map: maps.plain.clone_weak(),
            input_traffic_map: maps.traffic.clone_weak(),
            ..default()
        }));
}

fn param_update(
    mut query: Query<&Handle<MapVisualizerShader>>,
    crawlers_query: Query<&Crawler>,
    mut cs: ResMut<Assets<MapVisualizerShader>>,
) {
    let params = &mut cs.get_mut(query.single_mut()).unwrap().params;
    for (i, crawler) in crawlers_query.iter().enumerate() {
        params.crawlers[i] = crawler.to_gpu();
    }
}

#[derive(Asset, AsBindGroup, TypePath, Clone, Debug, Default)]
#[type_path = "shaders::map_visualizer_shader"]
struct MapVisualizerShader {
    #[uniform(0)]
    params: Params,
    #[storage_texture(1, access=ReadOnly)]
    input_plain_map: Handle<Image>,
    #[storage_texture(2, access=ReadOnly)]
    input_traffic_map: Handle<Image>,
}

#[derive(ShaderType, Clone, Debug)]
struct Params {
    crawlers: [CrawlerGPU; CRAWLERS_COUNT]
}

impl Default for Params {
    fn default() -> Self {
        Params {
            crawlers: [CrawlerGPU::default(); CRAWLERS_COUNT]
        }
    }
}

impl ComputeShader for MapVisualizerShader {
    fn shader() -> ShaderRef { 
        "shaders/map_visualizer_shader.wgsl".into()
    }

    fn entry_point() -> Cow<'static, str> {
        "update".into()
    }

    fn workgroups(texture_size: UVec2) -> UVec2 {
        texture_size / 8
    }
}
