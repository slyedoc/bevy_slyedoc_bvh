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
use std::{mem::swap, ops::Add, time::Duration};

pub mod prelude {
    pub use crate::{aabb::*, assets::*, bvh::*, camera::*, ray::*, tlas::*, tri::*, *};
}

const ROOT_NODE_IDX: usize = 0;
const BINS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(SystemLabel)]
pub enum BvhSystems {
    Setup,
    Camera,
}

pub struct BvhPlugin;
impl Plugin for BvhPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<BvhStats>()
            .init_resource::<Tlas>()
            .register_inspectable::<Bvh>()
            .register_inspectable::<BvhInstance>()
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
                    .with_system(CameraSystem::init_camera_image)
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
        query: Query<(Entity, &Handle<Mesh>), With<BvhInit>>,
        server: Res<AssetServer>,
        mut tlas: ResMut<Tlas>,
        mut stats: ResMut<BvhStats>,
    ) {
        for (e, handle) in query.iter() {
            let loaded = server.get_load_state(handle.id);
            let mesh = meshes.get(handle).expect("Mesh not found");
            let tris = parse_mesh(mesh);
            stats.tri_count += tris.len();

            let bvh_index = tlas.add_bvh(Bvh::new(tris));
            tlas.add_instance(BvhInstance {
                bvh_index,
                entity: Some(e),
                ..default()
            });

            commands.entity(e).remove::<BvhInit>();
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
        mut tlas: ResMut<Tlas>,
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

                    let bvh_index = tlas.add_bvh(Bvh::new(tris));
                    tlas.add_instance(BvhInstance {
                        bvh_index,
                        entity: Some(e),
                        ..default()
                    });
                }
            }

            commands.entity(root).remove::<BvhInitWithChildren>();
        }
    }

    pub fn update_bvh(mut query: Query<(&GlobalTransform)>, mut tlas: ResMut<Tlas>) {
        tlas.update_bvh(&query);
    }

    pub fn update_tlas(mut query: Query<(&GlobalTransform)>, mut tlas: ResMut<Tlas>) {
        // TODO: this is a hack, should only call once
        tlas.build();
    }

}

// Markers
#[derive(Component)]
pub struct BvhInit;
#[derive(Component)]
pub struct BvhInitWithChildren(pub Handle<Scene>);

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
