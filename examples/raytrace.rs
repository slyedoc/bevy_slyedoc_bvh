#![allow(warnings)]
mod helpers;
use std::borrow::Cow;

use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph},
        render_resource::*,
        renderer::{RenderContext, RenderDevice, RenderQueue},
        view::window,
        RenderApp, RenderStage,
    },
    window::{PresentMode, WindowResized},
};
use bevy_slyedoc_bvh::prelude::*;
use helpers::*;

const WORKGROUP_SIZE: u32 = 8;

fn main() {
    App::new()
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

fn setup(mut commands: Commands, mut image: Res<RtImage>, window: Res<WindowDescriptor>) {
    let size = UVec2::new(window.width as u32, window.height as u32);

    // create the image
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(size.x as f32, size.y as f32)),
                ..default()
            },
            texture: image.0.clone(),
            ..default()
        })
        .insert(RtSprite);

    commands.spawn_bundle(Camera2dBundle::default());
}

fn resize_sprite(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut sprite_query: Query<(&mut Sprite), With<RtSprite>>,
    mut rt_image: ResMut<RtImage>,
    mut resize_event: EventReader<WindowResized>,
) {
    for resize in resize_event.iter() {
        info!("resize {:?}", resize);
        let mut sprite = sprite_query.single_mut();

        sprite.custom_size = Some(Vec2::new(resize.width, resize.height));
    }
}

pub struct RaytracePlugin;

impl Plugin for RaytracePlugin {
    fn build(&self, app: &mut App) {
        // Extract the raytrace image resource from the main world into the render world
        // for operation on by the compute shader and display on the sprite.
        app.init_resource::<RtSettings>() 
            .init_resource::<RtImage>()                       
            .add_plugin(ExtractResourcePlugin::<RtImage>::default())
            .add_plugin(ExtractResourcePlugin::<RtSettings>::default())
            .add_system(Self::resize_window);

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
        mut rt_image: ResMut<RtImage>,
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

    fn queue_bind_group(
        mut commands: Commands,
        settings: Res<RtSettings>,
        pipeline: Res<RtPipeline>,
        gpu_images: Res<RenderAssets<Image>>,
        raytrace_image: Res<RtImage>,
        render_device: Res<RenderDevice>,
    ) {
         // image
        let view = &gpu_images[&raytrace_image.0];

        // settings
        let byte_buffer = [0u8; RtSettings::SIZE.get() as usize];
        let mut settings_buffer = encase::UniformBuffer::new(byte_buffer);
        settings_buffer.write(&settings.into_inner()).unwrap();

        let settings_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("rt settings uniform buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: settings_buffer.as_ref(),
        });

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &pipeline.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view.texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: settings_buffer.as_entire_binding(),
                },
            ],
        });
        commands.insert_resource(RtSettingsGpu {
            settings_buffer,
            bind_group,
        });
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
struct RtImage(Handle<Image>);

impl FromWorld for RtImage {
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
        RtImage(handle)
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
                                min_binding_size: Some(RtSettings::min_size()),                                
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
        let rt_bind_group = &world.resource::<RtSettingsGpu>().bind_group;
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
