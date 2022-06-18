use bevy::pbr::MaterialPipeline;
use bevy::reflect::TypeUuid;

use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::window::WindowResized;
use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    math::vec3,
    prelude::*,
    render::{
        render_asset::{PrepareAssetError, RenderAsset},
        render_resource::*,
        renderer::RenderDevice,
    },
};

use bvh::prelude::*;

pub struct RaytracePlugin;

impl Plugin for RaytracePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugin(BvhPlugin)
            .init_resource::<RtCamera>()
            .add_plugin(ExtractResourcePlugin::<RtCamera>::default())
            .add_plugin(MaterialPlugin::<RtMaterial>::default())
            .add_system(Self::update_camera)
            .add_system(Self::print_material);
    }
}

impl RaytracePlugin {
    fn print_material(
        query: &Query<(&RtMaterial)>,
    ) {

    }
    
    fn update_camera(
        query: Query<&Transform, With<Camera3d>>,
        mut rt_camera: ResMut<RtCamera>,
        mut resize_event: EventReader<WindowResized>,
    ) {
        for transform in query.iter() {
            rt_camera.origin = transform.translation;
            let camera = BvhCamera::new(1024, 1024);
            rt_camera.p0 = camera.get_ray(0.0, 0.0).direction;
            rt_camera.p1 = camera.get_ray(0.0, 1.0).direction;
            rt_camera.p2 = camera.get_ray(1.0, 0.0).direction;
        }

        for resize in resize_event.iter() {
            rt_camera.size.x = resize.width as u32;
            rt_camera.size.y = resize.height as u32;
        }
    }
}

#[derive(TypeUuid, Debug, Clone)]
#[uuid = "78a51cbd-381e-4d26-ad12-9527564cff9f"]
pub struct RtMaterial {
    pub background_texture: Handle<Image>,
}

pub struct GpuRtMaterial {
    _camera_buffer: Buffer,
    bind_group: BindGroup,
}

// The implementation of [`Material`] needs this impl to work properly.
impl RenderAsset for RtMaterial {
    type ExtractedAsset = RtMaterial;
    type PreparedAsset = GpuRtMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<MaterialPipeline<Self>>,
        SRes<RenderAssets<Image>>,
        SRes<RtCamera>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (render_device, material_pipeline, gpu_images, rt_camera): &mut SystemParamItem<
            Self::Param,
        >,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        
        let background_image = match gpu_images.get(&extracted_asset.background_texture) {
            Some(gpu_image) => gpu_image,
            // if the image isn't loaded yet, try next frame
            None => return Err(PrepareAssetError::RetryNextUpdate(extracted_asset)),
        };

        info!("background_image {:?}", background_image.texture_descriptor);

        let camera_buffer = {
            let byte_buffer = [0u8; RtCamera::SIZE.get() as usize];
            let mut buffer = encase::UniformBuffer::new(byte_buffer);
            let cam = rt_camera.clone();

            info!("camera {:?}", rt_camera);
            buffer.write(&cam).unwrap();
            render_device.create_buffer_with_data(&BufferInitDescriptor {
                contents: buffer.as_ref(),
                label: None,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            })
        };

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("rt bind group"),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&background_image.texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&background_image.sampler),
                },
            ],

            layout: &material_pipeline.material_layout,
        });

        Ok(GpuRtMaterial {
            _camera_buffer: camera_buffer,
            bind_group,
        })
    }
}

impl Material for RtMaterial {
    // When creating a custom material, you need to define either a vertex shader, a fragment shader or both.
    // If you don't define one of them it will use the default mesh shader which can be found at
    // <https://github.com/bevyengine/bevy/blob/latest/crates/bevy_pbr/src/render/mesh.wgsl>

    // For this example we don't need a vertex shader
    // fn vertex_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
    //     // Use the same path as the fragment shader since wgsl let's you define both shader in the same file
    //     Some(asset_server.load("shaders/custom_material.wgsl"))
    // }

    fn fragment_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        Some(asset_server.load("shaders/raytrace.wgsl"))
    }

    fn bind_group(render_asset: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup {
        &render_asset.bind_group
    }

    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(RtCamera::min_size()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
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
            label: None,
        })
    }
}

// TODO: Replace with view bindings
#[derive(ShaderType, Debug, Clone, ExtractResource)]
pub struct RtCamera {
    pub size: UVec2,
    origin: Vec3,
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
}

impl FromWorld for RtCamera {
    fn from_world(world: &mut World) -> Self {
        let window = world.resource::<WindowDescriptor>();
        let size = UVec2::new(window.width as u32, window.height as u32);
        RtCamera {
            size,
            origin: vec3(0.0, 0.0, -6.5),
            p0: vec3(-1.0, 1.0, 2.0),
            p1: vec3(1.0, 1.0, 2.0),
            p2: vec3(-1.0, -1.0, 2.0),
        }
    }
}

// pub struct RtPipeline {
//     bind_group_layout: BindGroupLayout,
//     init_pipeline: CachedComputePipelineId,
//     update_pipeline: CachedComputePipelineId,
// }

// impl FromWorld for RtPipeline {
//     fn from_world(world: &mut World) -> Self {
//         let bind_group_layout =
//             world
//                 .resource::<RenderDevice>()
//                 .create_bind_group_layout(&BindGroupLayoutDescriptor {
//                     label: Some("rt bind group layout"),
//                     entries: &[
//                         BindGroupLayoutEntry {
//                             binding: 0,
//                             visibility: ShaderStages::COMPUTE,
//                             ty: BindingType::StorageTexture {
//                                 access: StorageTextureAccess::ReadWrite,
//                                 format: TextureFormat::Rgba8Unorm,
//                                 view_dimension: TextureViewDimension::D2,
//                             },
//                             count: None,
//                         },
//                         BindGroupLayoutEntry {
//                             binding: 1,
//                             visibility: ShaderStages::COMPUTE,
//                             ty: BindingType::Buffer {
//                                 ty: BufferBindingType::Uniform,
//                                 has_dynamic_offset: false,
//                                 min_binding_size: Some(RtCamera::min_size()),
//                             },
//                             count: None,
//                         },
//                         BindGroupLayoutEntry {
//                             binding: 2,
//                             visibility: ShaderStages::COMPUTE,
//                             ty: BindingType::Buffer {
//                                 ty: BufferBindingType::Uniform,
//                                 has_dynamic_offset: false,
//                                 min_binding_size: Some(RtSize::min_size()),
//                             },
//                             count: None,
//                         },
//                         BindGroupLayoutEntry {
//                             binding: 3,
//                             visibility: ShaderStages::COMPUTE,
//                             ty: BindingType::StorageTexture {
//                                 access: StorageTextureAccess::ReadOnly,
//                                 format: TextureFormat::Rgba8Unorm,
//                                 view_dimension: TextureViewDimension::D2,
//                             },
//                             count: None,
//                         },
//                     ],
//                 });

//         let shader = world
//             .resource::<AssetServer>()
//             .load("shaders/raytrace.wgsl");

//         let mut pipeline_cache = world.resource_mut::<PipelineCache>();
//         let init_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
//             label: None,
//             layout: Some(vec![bind_group_layout.clone()]),
//             shader: shader.clone(),
//             shader_defs: vec![],
//             entry_point: Cow::from("init"),
//         });
//         let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
//             label: None,
//             layout: Some(vec![bind_group_layout.clone()]),
//             shader,
//             shader_defs: vec![],
//             entry_point: Cow::from("update"),
//         });

//         RtPipeline {
//             bind_group_layout,
//             init_pipeline,
//             update_pipeline,
//         }
//     }
// }
