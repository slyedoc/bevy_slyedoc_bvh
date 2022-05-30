#![allow(warnings)]
#![feature(let_chains)]
use bevy_inspector_egui::{
    plugin::InspectorWindows, Inspectable, InspectorPlugin, RegisterInspectable,
};
mod ray;
use rand::Rng;
use ray::*;
mod aabb;
use aabb::*;
mod tri;
use tri::*;
mod assets;
mod bvh;
use bvh::*;
mod camera;
mod tlas;
use tlas::*;

use bevy::{
    asset::LoadState,
    math::{vec2, vec3},
    prelude::*,
    reflect::TypeUuid,
    render::{
        camera::{Camera3d, CameraProjection},
        render_resource::{Extent3d, Texture, TextureDimension, TextureFormat},
    },
    tasks::ComputeTaskPool,
    transform::TransformSystem,
    utils::Instant,
};
use camera::*;
use rayon::prelude::*;
use std::{mem::swap, time::Duration};

pub mod prelude {
    pub use crate::{aabb::*, assets::*, bvh::*, camera::*, ray::*, tlas::*, tri::*, *};
}

const ROOT_NODE_IDX: usize = 0;
const BINS: usize = 8;

pub struct BvhPlugin;
impl Plugin for BvhPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Raycast>()
            .add_event::<RaycastResult>()
            //.add_plugin(InspectorPlugin::<BvhImage>::new())
            .init_resource::<BvhStats>()
            .add_asset::<Bvh>()
            .register_inspectable::<Bvh>()
            .register_inspectable::<Tlas>()
            .register_inspectable::<TlasNode>()
            .register_inspectable::<BvhCamera>()
            .register_inspectable::<BvhInstance>()
            .register_inspectable::<Tri>()
            .register_inspectable::<Aabb>()
            .add_system_set_to_stage(
                CoreStage::First,
                SystemSet::new()
                    .with_system(Self::spawn_bvh)
                    .with_system(Self::spawn_bvh_with_children),
            )
            .add_system_set_to_stage(
                CoreStage::PostUpdate,
                SystemSet::new()
                    .after(TransformSystem::TransformPropagate)
                    .with_system(Self::update_bvh_data)
                    .with_system(Self::run_raycasts.after(Self::update_bvh_data)),
            )
            // camera systems, will make into feature
            .add_system_set_to_stage(
                CoreStage::PostUpdate,
                SystemSet::new()
                    .with_system(CameraSystem::init_camera_image.after(Self::update_bvh_data))
                    .with_system(CameraSystem::update_camera.after(CameraSystem::init_camera_image))
                    .with_system(CameraSystem::render_camera.after(CameraSystem::update_camera)),
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
        mut bvhs: ResMut<Assets<Bvh>>,
        query: Query<(Entity, &Handle<Mesh>), With<BvhInit>>,
        server: Res<AssetServer>,
        mut stats: ResMut<BvhStats>,
    ) {
        for (e, handle) in query.iter() {
            let loaded = server.get_load_state(handle.id);
            let mesh = meshes.get(handle).expect("Mesh not found");
            let tris = parse_mesh(mesh);
            stats.tri_count += tris.len();

            commands
                .entity(e)
                .insert(BvhInstance {
                    bvh: bvhs.add(Bvh::new(tris)),
                    ..default()
                })
                .remove::<BvhInit>();
        }
    }

    fn spawn_bvh_with_children(
        mut commands: Commands,
        meshes: Res<Assets<Mesh>>,
        query: Query<(Entity, &BvhInitWithChildren)>,
        children: Query<(
            Entity,
            &GlobalTransform,
            Option<&Children>,
            Option<&Handle<Mesh>>,
        )>,
        server: Res<AssetServer>,
        mut stats: ResMut<BvhStats>,
        mut bvhs: ResMut<Assets<Bvh>>,
    ) {
        for (root, scene) in query.iter() {
            let load_state = server.get_load_state(scene.0.id);
            if load_state != LoadState::Loaded {
                continue;
            }

            let mut stack = vec![root];
            while let Some(e) = stack.pop() {
                let (e, trans, opt_children, opt_mesh) = children.get(e).unwrap();
                if let Some(children) = opt_children {
                    for child in children.iter() {
                        stack.push(*child);
                    }
                }
                if let Some(h_mesh) = opt_mesh {
                    let mesh = meshes.get(h_mesh).expect("Mesh not found");
                    let tris = parse_mesh(mesh);
                    stats.tri_count += tris.len();
                    commands
                        .entity(e)
                        .insert(BvhInstance {
                            bvh: bvhs.add(Bvh::new(tris)),
                            ..default()
                        });
                }
            }

            commands.entity(root).remove::<BvhInitWithChildren>();
        }
    }

    pub fn update_bvh_data(
        mut query: Query<(&mut BvhInstance, &GlobalTransform)>,
        bvhs: Res<Assets<Bvh>>,
    ) {
        for (mut bvh_inst, trans) in query.iter_mut() {
            // Update inv transfrom matrix for faster intersections
            let trans_matrix = trans.compute_matrix();
            bvh_inst.inv_trans = trans_matrix.inverse();

            // calculate world-space bounds using the new matrix
            let bvh = bvhs.get(bvh_inst.bvh.clone()).unwrap();
            let root = bvh.nodes[0];
            let bmin = root.aabb_min;
            let bmax = root.aabb_max;
            for i in 0..8 {
                bvh_inst.bounds.grow(trans_matrix.transform_point3(vec3(
                    if i & 1 != 0 { bmax.x } else { bmin.x },
                    if i & 2 != 0 { bmax.y } else { bmin.y },
                    if i & 4 != 0 { bmax.z } else { bmin.z },
                )));
            }
        }
    }

    pub fn run_raycasts(
        query: Query<(
            Entity,
            &GlobalTransform,
            &BvhInstance,
        )>,
        mut raycasts: EventReader<Raycast>,
        mut raycast_results: EventWriter<RaycastResult>,
        bvhs: Res<Assets<Bvh>>,
    ) {
        for raycast in raycasts.iter() {
            let mut target_entity = None;
            let mut ray = raycast.0;
            let mut tmp_distance = ray.t;

            for (e, _trans, bvh_inst) in query.iter() {
                //if ray.intersect_aabb(bounds.bmin, bounds.bmax) != 1e30f32 {
                bvh_inst.intersect(&mut ray, &bvhs);
                //}
                // TODO: just have interset return the closest intersection
                // We got closer, update target
                if tmp_distance != ray.t {
                    target_entity = Some(e);
                    tmp_distance = ray.t;
                }
            }

            raycast_results.send(RaycastResult {
                entity: if let Some(e) = target_entity {
                    Some(e)
                } else {
                    None
                },
                world_position: ray.origin + (ray.direction * ray.t),
                distance: ray.t,
            });
        }
    }
}

// Markers
#[derive(Component)]
pub struct BvhInit;

// TODO: make this a bit more generic, we only need the handle to check for loaded state
// maybe a better way to find that info
#[derive(Component)]
pub struct BvhInitWithChildren(pub Handle<Scene>);



// TODO: should be something like assets to do this
// TODO: will move into tlas

pub struct Raycast(pub Ray);
pub struct RaycastResult {
    pub entity: Option<Entity>,
    pub world_position: Vec3,
    pub distance: f32,
}

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
                verts[tri_indexes[0] as usize];
                let mut v0 = verts[tri_indexes[0] as usize];
                let mut v1 = verts[tri_indexes[1] as usize];
                let mut v2 = verts[tri_indexes[2] as usize];
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
