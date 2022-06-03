use bevy::{asset::LoadState, math::vec3, prelude::*, transform::TransformSystem};
use bevy_inspector_egui::prelude::*;

mod assets;
mod aabb;
use aabb::*;
mod bvh;
use bvh::*;
mod camera;
use camera::*;
mod ray;
mod tri;
use tri::*;
mod tlas;
use tlas::*;
use std::time::Duration;

pub mod prelude {
    pub use crate::{
        aabb::Aabb, assets::gen_random_triangles, bvh::*, camera::*, ray::*, tlas::*, tri::*,
        BvhInit, BvhPlugin, BvhSystems,
    };
}

pub fn test() {
    info!("test");
}

const BIN_COUNT: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemLabel)]
pub enum BvhSystems {
    Setup,
    Camera,
}

pub struct BvhPlugin;
impl Plugin for BvhPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BvhStats>()
            .init_resource::<Tlas>()
            .register_inspectable::<Bvh>()
            .register_inspectable::<BvhCamera>()
            .register_inspectable::<Tlas>()
            .register_inspectable::<TlasNode>()
            .register_inspectable::<Tri>()
            .register_inspectable::<Aabb>()
            .add_system_set_to_stage(
                CoreStage::PostUpdate,
                SystemSet::new()
                    .label(BvhSystems::Setup)
                    .after(TransformSystem::TransformPropagate)
                    .with_system(Self::spawn_bvh)
                    .with_system(Self::spawn_bvh_with_children)
                    .with_system(
                        Self::update_bvh
                            .after(Self::spawn_bvh)
                            .after(Self::spawn_bvh_with_children),
                    )
                    .with_system(Self::update_tlas.after(Self::update_bvh)),
            )
            // camera systems, will make into feature
            .add_system_set_to_stage(
                CoreStage::PostUpdate,
                SystemSet::new()
                    .label(BvhSystems::Camera)
                    .after(BvhSystems::Setup)
                    .with_system(camera_system::init_camera_image)
                    .with_system(
                        camera_system::update_camera.after(camera_system::init_camera_image),
                    )
                    .with_system(camera_system::render_camera.after(camera_system::update_camera)),
            );
    }
}

#[derive(Default)]
pub struct BvhStats {
    pub tri_count: usize,
    pub ray_count: f32,
    pub camera_time: Duration,
}

impl BvhPlugin {
    fn spawn_bvh(
        mut commands: Commands,
        meshes: Res<Assets<Mesh>>,
        query: Query<(Entity, &Handle<Mesh>), With<BvhInit>>,
        mut tlas: ResMut<Tlas>,
        mut stats: ResMut<BvhStats>,
    ) {
        for (e, handle) in query.iter() {
            // let loaded = server.get_load_state(handle.id);
            let mesh = meshes.get(handle).expect("Mesh not found");
            let tris = parse_mesh(mesh);
            stats.tri_count += tris.len();

            let bvh_index = tlas.add_bvh(Bvh::new(tris));
            tlas.add_instance(BvhInstance::new(e, bvh_index));
            commands.entity(e).remove::<BvhInit>();
        }
    }

    #[allow(clippy::type_complexity)]
    fn spawn_bvh_with_children(
        mut commands: Commands,
        meshes: Res<Assets<Mesh>>,
        query: Query<(Entity, &BvhInitWithChildren)>,
        children: Query<(Entity, Option<&Children>, Option<&Handle<Mesh>>)>,
        server: Res<AssetServer>,
        mut stats: ResMut<BvhStats>,
        mut tlas: ResMut<Tlas>,
    ) {
        for (root, scene) in query.iter() {
            let load_state = server.get_load_state(scene.0.id);
            if load_state != LoadState::Loaded {
                continue;
            }

            let mut stack = vec![root];
            while let Some(e) = stack.pop() {
                let (e, opt_children, opt_mesh) = children.get(e).unwrap();
                if let Some(children) = opt_children {
                    for child in children.iter() {
                        stack.push(*child);
                    }
                }
                if let Some(h_mesh) = opt_mesh {
                    let mesh = meshes.get(h_mesh).expect("Mesh not found");
                    let tris = parse_mesh(mesh);
                    stats.tri_count += tris.len();

                    let bvh_index = tlas.add_bvh(Bvh::new(tris));
                    tlas.add_instance(BvhInstance::new(e, bvh_index));
                }
            }

            commands.entity(root).remove::<BvhInitWithChildren>();
        }
    }

    // TODO: both of these update system are incomplete, for now we are rebuilding every frame
    // for now working on speeding up ray intersection
    // will come back to this
    pub fn update_bvh(query: Query<&GlobalTransform>, mut tlas: ResMut<Tlas>) {
        // moved fn into tlas self to since it needed 2 mutable refs within the tlas
        tlas.update_bvh_instances(&query);
    }

    pub fn update_tlas(mut tlas: ResMut<Tlas>) {
        tlas.build();
    }
}

pub mod camera_system {
    use bevy::{
        math::vec3,
        prelude::*,
        render::render_resource::{Extent3d, TextureDimension, TextureFormat},
        utils::Instant,
    };
    use rayon::prelude::*;

    use crate::{ray::Ray, tlas::Tlas, BvhStats};

    use super::BvhCamera;

    pub fn init_camera_image(
        mut query: Query<&mut BvhCamera, Added<BvhCamera>>,
        mut images: ResMut<Assets<Image>>,
    ) {
        for mut camera in query.iter_mut() {
            let image = images.add(Image::new(
                Extent3d {
                    width: camera.width as u32,
                    height: camera.height as u32,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                vec![0; (camera.width * camera.height) as usize * 4],
                TextureFormat::Rgba8UnormSrgb,
            ));
            camera.image = Some(image);
        }
    }

    pub fn update_camera(mut camera_query: Query<(&mut BvhCamera, &GlobalTransform)>) {
        for (mut camera, trans) in camera_query.iter_mut() {
            camera.update(trans);
        }
    }

    pub fn render_camera(
        camera_query: Query<&BvhCamera>,
        mut images: ResMut<Assets<Image>>,
        mut stats: ResMut<BvhStats>,
        tlas: Res<Tlas>,
    ) {
        if let Ok(camera) = camera_query.get_single() {
            if let Some(image) = &camera.image {
                let start = Instant::now();
                let image = images.get_mut(image).unwrap();

                // TODO: Make this acutally tilings, currenty this just takes a slice pixels in a row
                const PIXEL_TILE_COUNT: usize = 64;
                const PIXEL_TILE: usize = 4 * PIXEL_TILE_COUNT;
                image
                    .data
                    .par_chunks_mut(PIXEL_TILE)
                    .enumerate()
                    .for_each(|(i, pixels)| {
                        let mut ray = Ray::default();
                        for pixel_offset in 0..(pixels.len() / 4) {
                            let index = i * PIXEL_TILE_COUNT + pixel_offset;
                            let offset = pixel_offset * 4;

                            let x = index as u32 % camera.width;
                            let y = index as u32 / camera.width;
                            let u = x as f32 / camera.width as f32;
                            let v = y as f32 / camera.height as f32;
                            // TODO: Revisit multiple samples later
                            // if samples > 0 {
                            //     u += rng.gen::<f32>() / camera.width as f32;
                            //     v += rng.gen::<f32>() / camera.height as f32;
                            // }

                            // TODO: flip v since image is upside down, figure out why
                            let mut ray = camera.get_ray(u, 1.0 - v);
                            let color = if let Some(hit) = ray.intersect_tlas(&tlas) {
                                vec3(hit.u, hit.v, 1.0 - (hit.u + hit.v)) * 255.0
                            } else {
                                Vec3::ZERO
                            };

                            pixels[offset] = color.x as u8;
                            pixels[offset + 1] = color.y as u8;
                            pixels[offset + 2] = color.z as u8;
                            pixels[offset + 3] = 255;
                        }
                    });

                stats.ray_count =
                    camera.width as f32 * camera.height as f32 * camera.samples as f32;
                stats.camera_time = start.elapsed();
            }
        }
    }
}

// Markers
#[derive(Component)]
pub struct BvhInit;
#[derive(Component)]
pub struct BvhInitWithChildren(pub Handle<Scene>);

// TODO: We dont really want to copy the all tris, find better way
pub fn parse_mesh(mesh: &Mesh) -> Vec<Tri> {
    match mesh.primitive_topology() {
        bevy::render::mesh::PrimitiveTopology::TriangleList => {
            let indexes = match mesh.indices().expect("No Indices") {
                bevy::render::mesh::Indices::U32(vec) => vec,
                _ => todo!(),
            };

            let verts = match mesh
                .attribute(Mesh::ATTRIBUTE_POSITION)
                .expect("No Position Attribute")
            {
                bevy::render::mesh::VertexAttributeValues::Float32x3(vec) => {
                    vec.iter().map(|vec| vec3(vec[0], vec[1], vec[2]))
                }
                _ => todo!(),
            }
            .collect::<Vec<_>>();

            let mut triangles = Vec::with_capacity(indexes.len() / 3);
            for tri_indexes in indexes.chunks(3) {
                let v0 = verts[tri_indexes[0] as usize];
                let v1 = verts[tri_indexes[1] as usize];
                let v2 = verts[tri_indexes[2] as usize];
                triangles.push(Tri::new(
                    vec3(v0[0], v0[1], v0[2]),
                    vec3(v1[0], v1[1], v1[2]),
                    vec3(v2[0], v2[1], v2[2]),
                ));
            }
            triangles
        }
        _ => todo!(),
    }
}
