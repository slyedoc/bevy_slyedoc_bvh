#![allow(warnings)]
mod helpers;
use std::f32::consts::PI;

use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    pbr::MaterialPipeline,
    prelude::*,
    reflect::TypeUuid,
    render::{camera::ScalingMode, render_asset::*, render_resource::*, renderer::*},
    sprite::{Material2d, Material2dPipeline, Material2dPlugin, MaterialMesh2dBundle},
    window::PresentMode,
};

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
            title: "Shader".to_string(),
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
        .add_plugin(MaterialPlugin::<BvhMaterial>::default())
        //.add_plugin(DebugLinesPlugin::default())
        .add_startup_system(spawn_camera)
        //.add_startup_system(helpers::setup_cameras)
        .add_startup_system(helpers::load_enviroment)
        //.add_startup_system(helpers::load_sponza)
        .add_startup_system(load_image)
        .add_system(spawn_quad)
        .run();
}

fn load_image(mut commands: Commands, assets: Res<AssetServer>) {
    let awesome = assets.load("images/awesome.png");
    commands.insert_resource(Awesome(awesome));
}

fn spawn_camera(mut commands: Commands) {
    // camera
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    })
    .insert(CameraController::default());
}

fn spawn_quad(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut bvh_material: ResMut<Assets<BvhMaterial>>,
    
    awesome: Res<Awesome>,
) {
    commands.spawn_bundle(MaterialMeshBundle {
        transform: Transform::from_xyz(0.0, 1.0, 0.0),
        mesh: meshes.add(Mesh::from(shape::Quad::default())).into(),
        material: bvh_material.add(BvhMaterial { 
            color: Color::RED,  
            mouse: Vec2::new(0.0, 0.0),
        }),
        ..default()
    });
}

#[derive(TypeUuid, Clone)]
#[uuid = "90634fdb-f9e1-41b9-85b9-fc4d2979cd09"]
struct BvhMaterial {
    pub color: Color,
    pub mouse: Vec2,
}

#[derive(Clone)]
pub struct GpuBvhMaterial {
    _buffer: Buffer,
    bind_group: BindGroup,
}

// The implementation of [`Material`] needs this impl to work properly.
impl RenderAsset for BvhMaterial {
    type ExtractedAsset = BvhMaterial;
    type PreparedAsset = GpuBvhMaterial;
    type Param = (SRes<RenderDevice>, SRes<MaterialPipeline<Self>>);
    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (render_device, material_pipeline): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let color = Vec4::from_slice(&extracted_asset.color.as_linear_rgba_f32());

        let byte_buffer = [0u8; Vec4::SIZE.get() as usize];
        let mut buffer = encase::UniformBuffer::new(byte_buffer);
        buffer.write(&color).unwrap();

        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: buffer.as_ref(),
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: None,
            layout: &material_pipeline.material_layout,
        });

        Ok(GpuBvhMaterial {
            _buffer: buffer,
            bind_group,
        })
    }
}

impl Material for BvhMaterial {
    // When creating a custom material, you need to define either a vertex shader, a fragment shader or both.
    // If you don't define one of them it will use the default mesh shader which can be found at
    // <https://github.com/bevyengine/bevy/blob/latest/crates/bevy_pbr/src/render/mesh.wgsl>

    // For this example we don't need a vertex shader
    // fn vertex_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
    //     // Use the same path as the fragment shader since wgsl let's you define both shader in the same file
    //     Some(asset_server.load("shaders/custom_material.wgsl"))
    // }

    fn fragment_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        Some(asset_server.load("shaders/bvh_material.wgsl"))
    }

    fn bind_group(render_asset: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup {
        &render_asset.bind_group
    }

    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(Vec4::min_size()),
                },
                count: None,
            }],
            label: None,
        })
    }
}