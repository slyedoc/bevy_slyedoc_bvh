#![allow(warnings)]
mod helpers;
use std::borrow::Cow;

use bevy::{
    math::vec3,
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph},
        render_resource::*,
        renderer::{RenderContext, RenderDevice, RenderQueue},
        view::window,
        RenderApp, RenderStage, settings::WgpuSettings,
    },
    window::{PresentMode, WindowResized}, asset::AssetLoader,
};
use bevy_slyedoc_bvh::prelude::*;
use helpers::*;

const WORKGROUP_SIZE: u32 = 8;

fn main() {
    App::new()
        // .insert_resource(WgpuSettings {
        //     //features: WgpuFeatures::default(),
        //     ..default()
        // })
        .insert_resource(WindowDescriptor {
            present_mode: PresentMode::Fifo,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(HelperPlugin) // See cusor plugin in helper plugins
        .add_plugin(BvhPlugin)
        .add_plugin(RaytracePlugin)
        //.add_startup_system(helpers::setup_cameras)
        .add_startup_system(helpers::load_enviroment)
        //.add_startup_system(helpers::load_sponza)
        .add_startup_system(setup)
        .add_system(resize_sprite)
        .run();
}

// Marker for Sprite for resize
#[derive(Component)]
struct RtSprite;

fn setup(
    mut commands: Commands,
    mut rt_image: Res<RtImageOut>,
    mut rt_background: Res<RtBackground>,
    window: Res<WindowDescriptor>,
) {
    let size = UVec2::new(window.width as u32, window.height as u32);

    // create the image
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(size.x as f32, size.y as f32)),
                ..default()
            },
            texture: rt_image.0.clone(),
            ..default()
        })
        .insert(RtSprite);
    commands.spawn_bundle(Camera2dBundle::default());

    // Hack to get texture into gpu_images
    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(300.0, 300.0)),
            ..default()
        },
        texture: rt_background.0.clone(),
        ..default()
    });
}

fn resize_sprite(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut sprite_query: Query<(&mut Sprite), With<RtSprite>>,
    mut rt_image: ResMut<RtImageOut>,
    mut resize_event: EventReader<WindowResized>,
) {
    for resize in resize_event.iter() {
        //info!("resize {:?}", resize);
        let mut sprite = sprite_query.single_mut();
        sprite.custom_size = Some(Vec2::new(resize.width, resize.height));
    }
}

// TODO: move this to lib, here for testing
pub struct RaytracePlugin;

impl Plugin for RaytracePlugin {
    fn build(&self, app: &mut App) {
        // Extract the raytrace image resource from the main world into the render world
        // for operation on by the compute shader and display on the sprite.
        app.init_resource::<RtSettings>()
            .init_resource::<RtImageOut>()            
            .init_resource::<RtBackground>()     
            .init_resource::<RtCamera>()
            // .add_startup_system_to_stage(StartupStage::PreStartup, load_background)
            .add_plugin(ExtractResourcePlugin::<RtImageOut>::default())
            .add_plugin(ExtractResourcePlugin::<RtBackground>::default())
            .add_plugin(ExtractResourcePlugin::<RtCamera>::default())
            .add_plugin(ExtractResourcePlugin::<RtSettings>::default())
            .add_system(Self::resize_window)
            .add_system(background_loaded);

        let render_device = app.world.resource::<RenderDevice>();

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<RtPipeline>()
            .add_system_to_stage(RenderStage::Queue, Self::queue_bind_group);

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node("raytrace", RtNode::default());
        render_graph
            .add_node_edge("raytrace", bevy::render::main_graph::node::CAMERA_DRIVER)
            .unwrap();
    }
}

impl RaytracePlugin {
    fn resize_window(
        mut rt_settings: ResMut<RtSettings>,
        mut rt_image: ResMut<RtImageOut>,
        mut images: ResMut<Assets<Image>>,
        mut resize_event: EventReader<WindowResized>,
    ) {
        for resize in resize_event.iter() {
            rt_settings.size.x = resize.width as u32;
            rt_settings.size.y = resize.height as u32;

            // TODO: This doesnt work, handle gets updated correctly, but image isnt displayed
            // Has something to do with the binding
            // let mut image = Image::new_fill(
            //     Extent3d {
            //         width: rt_settings.size.x,
            //         height: rt_settings.size.y,
            //         depth_or_array_layers: 1,
            //     },
            //     TextureDimension::D2,
            //     &[0, 0, 0, 255],
            //     TextureFormat::Rgba8Unorm,
            // );
            // image.texture_descriptor.usage = TextureUsages::COPY_DST
            //     | TextureUsages::STORAGE_BINDING
            //     | TextureUsages::TEXTURE_BINDING;

            // rt_image.0 = images.add(image);
        }
    }

    fn extract_camera(commands: &mut Commands, query: Query<(&Transform), With<Camera3d>>) {
        todo!();
    }

    // TODO: reuse buffers, for now each frame a new one is created
    // having hard enough time as it is
    fn queue_bind_group(
        mut commands: Commands,
        render_device: Res<RenderDevice>,
        pipeline: Res<RtPipeline>,
        gpu_images: Res<RenderAssets<Image>>,
        rt_image: Res<RtImageOut>,
        rt_background: Res<RtBackground>,
        settings: Res<RtSettings>,
        camera: Res<RtCamera>,
    ) {
        // output image
        let image_view = &gpu_images[&rt_image.0];
        // background image
        let background = gpu_images.get(&rt_background.0);
        if background.is_none() {
            warn!("background image not found");
            return;
        }
        let background_view = &gpu_images[&rt_background.0];

        // settings
        let settings_buffer = {
            let byte_buffer = [0u8; RtSettings::SIZE.get() as usize];
            let mut encase_settings_buffer = encase::UniformBuffer::new(byte_buffer);
            encase_settings_buffer
                .write(&settings.into_inner())
                .unwrap();
            render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("rt settings uniform buffer"),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                contents: encase_settings_buffer.as_ref(),
            })
        };

        // camera
        let camera_buffer = {
            let byte_buffer = [0u8; RtCamera::SIZE.get() as usize];
            let mut buffer = encase::UniformBuffer::new(byte_buffer);
            buffer.write(&camera.into_inner()).unwrap();
            render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("rt camera uniform buffer"),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                contents: buffer.as_ref(),
            })
        };

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &pipeline.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&image_view.texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: camera_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: settings_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&background_view.texture_view),
                },
            ],
        });

        commands.insert_resource(RtSettingsGpu {
            settings_buffer,
            bind_group,
        });
    }
}

#[derive(ShaderType, Clone, ExtractResource)]
struct RtCamera {
    origin: Vec3,
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
}

impl Default for RtCamera {
    fn default() -> Self {
        RtCamera {
            // TODO: bases this on the 3d camera
            origin: vec3(0.0, 0.0, -6.5),
            p0: vec3(-1.0, 1.0, 2.0),
            p1: vec3(1.0, 1.0, 2.0),
            p2: vec3(-1.0, -1.0, 2.0),
        }
    }
}

#[derive(ShaderType, Debug, Clone, ExtractResource)]
struct RtSettings {
    pub size: UVec2,
}

impl FromWorld for RtSettings {
    fn from_world(world: &mut World) -> Self {
        let window = world.resource::<WindowDescriptor>();
        let size = UVec2::new(window.width as u32, window.height as u32);
        RtSettings { size }
    }
}
#[derive(Clone, Deref, ExtractResource)]
struct RtImageOut(Handle<Image>);

impl FromWorld for RtImageOut {
    fn from_world(world: &mut World) -> Self {
        let settings = world.resource::<RtSettings>();
        let mut image = Image::new_fill(
            Extent3d {
                width: settings.size.x,
                height: settings.size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8Unorm,
        );
        image.texture_descriptor.usage = TextureUsages::COPY_DST
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING;
        let mut images = world.resource_mut::<Assets<Image>>();
        let handle = images.add(image);
        RtImageOut(handle)
    }
}


fn background_loaded(
    mut commands: Commands,
    server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    bg: Res<RtBackground>,
) {
    use bevy::asset::LoadState;

    let handle = bg.0.clone_untyped();
    match server.get_load_state(handle) {
        LoadState::Failed => {
            // one of our assets had an error
            //warn!("background failed")
        }
        LoadState::Loaded => {
            // all assets are now ready
            info!("background loaded");
            let img = images.get(&bg.0).unwrap();
            info!(" {:?}", img.texture_descriptor);
            // this might be a good place to transition into your in-game state

            // remove the resource to drop the tracking handles
                    // (note: if you don't have any other handles to the assets
            // elsewhere, they will get unloaded after this)
        }
        LoadState::NotLoaded => {
            // one of our assets is still loading
            //info!("background not loaded");
        },
        LoadState::Loading => {
            //info!("background loading");
        },
        LoadState::Unloaded => {
            //info!("background unloaded");
        },
        
    }
  
}
#[derive(Clone, Deref, ExtractResource)]
struct RtBackground(Handle<Image>);

impl FromWorld for RtBackground {
    fn from_world(world: &mut World) -> Self {
        let settings = world.resource::<RtSettings>();
        let mut image = Image::new_fill(
            Extent3d {
                width: settings.size.x,
                height: settings.size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[100, 100, 100, 255],
            TextureFormat::Rgba8Unorm,
        );
        image.texture_descriptor.usage = TextureUsages::COPY_DST
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING;

        let mut images = world.resource_mut::<Assets<Image>>();
        let handle = images.add(image);
        RtBackground(handle)

        // let images = world.resource::<Assets<Image>>();
        // let asset_server = world.resource_mut::<AssetServer>();
        // let handle: Handle<Image> = asset_server.load("images/awesome.png");
        // RtBackground(handle)
    }
}

struct RtSettingsGpu {
    settings_buffer: Buffer,
    bind_group: BindGroup,
}

pub struct RtPipeline {
    bind_group_layout: BindGroupLayout,
    init_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId,
}

impl FromWorld for RtPipeline {
    fn from_world(world: &mut World) -> Self {
        let bind_group_layout =
            world
                .resource::<RenderDevice>()
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("rt bind group layout"),
                    entries: &[
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::StorageTexture {
                                access: StorageTextureAccess::ReadWrite,
                                format: TextureFormat::Rgba8Unorm,
                                view_dimension: TextureViewDimension::D2,
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: Some(RtCamera::min_size()),
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 2,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: Some(RtSettings::min_size()),
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 3,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::StorageTexture {
                                access: StorageTextureAccess::ReadOnly,
                                format: TextureFormat::Rgba8Unorm,
                                view_dimension: TextureViewDimension::D2,
                            },
                            count: None,
                        },
                    ],
                });

        let shader = world
            .resource::<AssetServer>()
            .load("shaders/raytrace.wgsl");

        let mut pipeline_cache = world.resource_mut::<PipelineCache>();
        let init_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: Some(vec![bind_group_layout.clone()]),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("init"),
        });
        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: Some(vec![bind_group_layout.clone()]),
            shader,
            shader_defs: vec![],
            entry_point: Cow::from("update"),
        });

        RtPipeline {
            bind_group_layout,
            init_pipeline,
            update_pipeline,
        }
    }
}

enum RtState {
    Loading,
    Init,
    Update,
}

struct RtNode {
    state: RtState,
}

impl Default for RtNode {
    fn default() -> Self {
        Self {
            state: RtState::Loading,
        }
    }
}

impl render_graph::Node for RtNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<RtPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        // if the corresponding pipeline has loaded, transition to the next stage
        match self.state {
            RtState::Loading => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.init_pipeline)
                {
                    self.state = RtState::Init;
                }
            }
            RtState::Init => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline)
                {
                    self.state = RtState::Update;
                }
            }
            RtState::Update => {}
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let settings = world.get_resource::<RtSettingsGpu>();
        if settings.is_none() {
            return Ok(());
        }
        let rt_bind_group = &settings.unwrap().bind_group;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<RtPipeline>();
        let rt_settings = world.resource::<RtSettings>();

        let mut pass = render_context
            .command_encoder
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, rt_bind_group, &[]);

        // select the pipeline based on the current state
        match self.state {
            RtState::Loading => {}
            RtState::Init => {
                let init_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.init_pipeline)
                    .unwrap();
                pass.set_pipeline(init_pipeline);
                pass.dispatch(
                    rt_settings.size.x / WORKGROUP_SIZE,
                    rt_settings.size.y / WORKGROUP_SIZE,
                    1,
                );
            }
            RtState::Update => {
                let update_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.update_pipeline)
                    .unwrap();
                pass.set_pipeline(update_pipeline);
                pass.dispatch(
                    rt_settings.size.x / WORKGROUP_SIZE,
                    rt_settings.size.y / WORKGROUP_SIZE,
                    1,
                );
            }
        }

        Ok(())
    }
}
