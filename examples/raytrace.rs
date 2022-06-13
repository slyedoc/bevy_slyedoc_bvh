#![allow(warnings)]
mod helpers;
use std::borrow::Cow;

use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph},
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
            CachedComputePipelineId, CachedPipelineState, ComputePassDescriptor,
            ComputePipelineDescriptor, Extent3d, PipelineCache, ShaderStages, StorageTextureAccess,
            TextureDimension, TextureFormat, TextureUsages, TextureViewDimension, BufferBindingType, BufferSize, ShaderType,
        },
        renderer::{RenderContext, RenderDevice},
        RenderApp, RenderStage,
    },
    window::PresentMode,
};
use bevy_slyedoc_bvh::prelude::*;
use helpers::*;

const SIZE: (u32, u32) = (1280, 720);
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
        //.add_plugin(DebugLinesPlugin::default())
        //.add_startup_system(helpers::setup_cameras)
        .add_startup_system(helpers::load_enviroment)
        //.add_startup_system(helpers::load_sponza)
        .add_plugin(RaytracePlugin)
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut image = Image::new_fill(
        Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
    );
    image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    let image = images.add(image);

    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(SIZE.0 as f32, SIZE.1 as f32)),
            ..default()
        },
        texture: image.clone(),
        ..default()
    });
    commands.spawn_bundle(Camera2dBundle::default());

    commands.insert_resource(RaytraceImage(image));
}

pub struct RaytracePlugin;

impl Plugin for RaytracePlugin {
    fn build(&self, app: &mut App) {
        // Extract the raytrace image resource from the main world into the render world
        // for operation on by the compute shader and display on the sprite.
        app.add_plugin(ExtractResourcePlugin::<RaytraceImage>::default());
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<RaytracePipeline>()
            .add_system_to_stage(RenderStage::Queue, queue_bind_group);

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node("raytrace", RaytraceNode::default());
        render_graph
            .add_node_edge("raytrace", bevy::render::main_graph::node::CAMERA_DRIVER)
            .unwrap();
    }
}

#[derive(ShaderType)]
struct RaytraceSettings {
    pub size: UVec2,
}

#[derive(Clone, Deref, ExtractResource)]
struct RaytraceImage(Handle<Image>);

struct RaytraceBindGroup(BindGroup);

fn queue_bind_group(
    mut commands: Commands,
    pipeline: Res<RaytracePipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    raytrace_image: Res<RaytraceImage>,
    render_device: Res<RenderDevice>,
) {
    let view = &gpu_images[&raytrace_image.0];
    let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &pipeline.texture_bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&view.texture_view),
            },
        ],
    });
    commands.insert_resource(RaytraceBindGroup(bind_group));
}

pub struct RaytracePipeline {
    texture_bind_group_layout: BindGroupLayout,
    init_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId,
}

impl FromWorld for RaytracePipeline {
    fn from_world(world: &mut World) -> Self {
        let texture_bind_group_layout =
            world
                .resource::<RenderDevice>()
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::ReadWrite,
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
            layout: Some(vec![texture_bind_group_layout.clone()]),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("init"),
        });
        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: Some(vec![texture_bind_group_layout.clone()]),
            shader,
            shader_defs: vec![],
            entry_point: Cow::from("update"),
        });

        RaytracePipeline {
            texture_bind_group_layout,
            init_pipeline,
            update_pipeline,
        }
    }
}

enum RaytraceState {
    Loading,
    Init,
    Update,
}

struct RaytraceNode {
    state: RaytraceState,
}

impl Default for RaytraceNode {
    fn default() -> Self {
        Self {
            state: RaytraceState::Loading,
        }
    }
}

impl render_graph::Node for RaytraceNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<RaytracePipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        // if the corresponding pipeline has loaded, transition to the next stage
        match self.state {
            RaytraceState::Loading => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.init_pipeline)
                {
                    self.state = RaytraceState::Init;
                }
            }
            RaytraceState::Init => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline)
                {
                    self.state = RaytraceState::Update;
                }
            }
            RaytraceState::Update => {}
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let texture_bind_group = &world.resource::<RaytraceBindGroup>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<RaytracePipeline>();

        let mut pass = render_context
            .command_encoder
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, texture_bind_group, &[]);

        // select the pipeline based on the current state
        match self.state {
            RaytraceState::Loading => {}
            RaytraceState::Init => {
                let init_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.init_pipeline)
                    .unwrap();
                pass.set_pipeline(init_pipeline);
                pass.dispatch(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
            }
            RaytraceState::Update => {
                let update_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.update_pipeline)
                    .unwrap();
                pass.set_pipeline(update_pipeline);
                pass.dispatch(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
            }
        }

        Ok(())
    }
}
