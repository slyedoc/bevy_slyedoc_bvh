#![allow(warnings)]
mod helpers;
use std::f32::consts::PI;

use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_asset::*,
        render_resource::{
            std140::{AsStd140, Std140},
            *,
        },
        renderer::*, camera::ScalingMode,
    },
    sprite::{Material2d, Material2dPipeline, Material2dPlugin, MaterialMesh2dBundle},
    window::PresentMode,
};
use bevy_prototype_debug_lines::{DebugLines, DebugLinesPlugin};
use bevy_slyedoc_bvh::prelude::*;
use helpers::*;

#[derive(Deref)]
pub struct Awesome(Handle<Image>);

pub const CLEAR: Color = Color::rgb(0.3, 0.3, 0.3);
pub const HEIGHT: f32 = 900.0;
pub const RESOLUTION: f32 = 16.0 / 9.0;

fn main() {
    App::new()
        .insert_resource(ClearColor(CLEAR))
        .insert_resource(WindowDescriptor {
            width: HEIGHT * RESOLUTION,
            height: HEIGHT,
            title: "Bevy Template".to_string(),
            present_mode: PresentMode::Fifo,
            resizable: false,
            ..Default::default()
        })
        .insert_resource(WindowDescriptor {
            present_mode: PresentMode::Fifo,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(HelperPlugin) // See cusor plugin in helper plugins
        .add_plugin(BvhPlugin)
        .add_plugin(Material2dPlugin::<BvhMaterial>::default())
        //.add_plugin(DebugLinesPlugin::default())
        .add_startup_system(spawn_camera)
        //.add_startup_system(helpers::setup_cameras)
        //.add_startup_system(helpers::load_enviroment)
        //.add_startup_system(helpers::load_sponza)
        .add_startup_system(load_image)
        .add_system(spawn_quad)
        .run();
}

fn spawn_quad(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut bvh_material: ResMut<Assets<BvhMaterial>>,
    awesome: Res<Awesome>,
) {
    commands.spawn_bundle(MaterialMesh2dBundle {
        mesh: meshes.add(Mesh::from(shape::Quad::default())).into(),
        material: bvh_material.add(BvhMaterial {
            alpha: 0.5,
            color: Color::RED,
            image: awesome.clone(),
        }),
        ..default()
    });
}

#[derive(TypeUuid, Clone)]
#[uuid = "90634fdb-f9e1-41b9-85b9-fc4d2979cd09"]
struct BvhMaterial {
    pub alpha: f32,
    pub color: Color,
    pub image: Handle<Image>,
}

#[derive(Clone, AsStd140)]
struct BvhMaterialUniformData {
    pub alpha: f32,
    pub color: Vec4,
}

struct BvhMaterialGPU {
    bind_group: BindGroup,
}

impl Material2d for BvhMaterial {
    fn bind_group_layout(render_device: &bevy::render::renderer::RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        min_binding_size: BufferSize::new(
                            BvhMaterialUniformData::std140_size_static() as u64,
                        ),
                        has_dynamic_offset: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }

    fn bind_group(material: &BvhMaterialGPU) -> &BindGroup {
        &material.bind_group
    }

    fn fragment_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        asset_server.watch_for_changes().unwrap();
        Some(asset_server.load("shaders/bvh_material.wgsl"))
    }
}

impl RenderAsset for BvhMaterial {
    type ExtractedAsset = BvhMaterial;
    type PreparedAsset = BvhMaterialGPU;
    type Param = (
        SRes<RenderDevice>,
        SRes<Material2dPipeline<BvhMaterial>>,
        SRes<RenderAssets<Image>>,
    );

    fn prepare_asset(
        extracted_asset: BvhMaterial,
        (render_device, pipeline, images): &mut SystemParamItem<Self::Param>,
    ) -> Result<BvhMaterialGPU, PrepareAssetError<BvhMaterial>> {
        let uniform_data = BvhMaterialUniformData {
            alpha: extracted_asset.alpha,
            color: extracted_asset.color.as_linear_rgba_f32().into(),
        };

        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: None,
            contents: uniform_data.as_std140().as_bytes(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let (view, sampler) = if let Some(result) = pipeline
            .mesh2d_pipeline
            .get_image_texture(images, &Some(extracted_asset.image.clone()))
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(extracted_asset));
        };

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &pipeline.material2d_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(sampler),
                },
            ],
        });
        Ok(BvhMaterialGPU { bind_group })
    }

    fn extract_asset(&self) -> BvhMaterial {
        self.clone()
    }
}

fn load_image(mut commands: Commands, assets: Res<AssetServer>) {
    let awesome = assets.load("images/awesome.png");
    commands.insert_resource(Awesome(awesome));
}

fn spawn_camera(mut commands: Commands) {
    let mut camera = OrthographicCameraBundle::new_2d();

     camera.orthographic_projection.right = 1.0 * RESOLUTION;
     camera.orthographic_projection.left = -1.0 * RESOLUTION;

    camera.orthographic_projection.top = 1.0;
    camera.orthographic_projection.bottom = -1.0;

    camera.orthographic_projection.scaling_mode = ScalingMode::None;

    commands.spawn_bundle(camera);
}
