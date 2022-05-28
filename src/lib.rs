#![allow(warnings)]
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use image::{Rgb, RgbImage};
mod ray;
use ray::*;
mod aabb;
use aabb::*;
mod tri;
use tri::*;
mod assets;
mod bvh;
mod camera;
use bvh::*;

use bevy::{
    asset::LoadState, math::vec3, prelude::*, reflect::TypeUuid, transform::TransformSystem,
};
use std::mem::swap;

pub mod prelude {
    pub use crate::{aabb::*, assets::*, bvh::*, camera::*, ray::*, tri::*, *};
}

const ROOT_NODE_IDX: usize = 0;
const BINS: usize = 8;

pub struct BvhPlugin;
impl Plugin for BvhPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Raycast>()
            .add_event::<RaycastResult>()
            .init_resource::<BvhVec>()
            .register_inspectable::<Bvh>()
            .register_inspectable::<Tris>()
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
            );
    }
}

impl BvhPlugin {
    pub fn spawn_bvh(
        mut commands: Commands,
        meshes: Res<Assets<Mesh>>,
        query: Query<(Entity, &Handle<Mesh>), Added<BvhInit>>,
        mut bvhs: ResMut<BvhVec>,
    ) {
        for (e, handle) in query.iter() {
            let mesh = meshes.get(handle).expect("Mesh not found");
            let tris = parse_mesh(mesh);
            commands
                .entity(e)
                .insert(bvhs.add(Bvh::new(&tris)))
                .insert(Tris(tris))
                .insert(InvTrans(Mat4::ZERO))
                .insert(Aabb::default())
                .remove::<BvhInit>();
        }
    }

    pub fn spawn_bvh_with_children(
        mut commands: Commands,
        meshes: Res<Assets<Mesh>>,
        query: Query<(Entity, &BvhInitWithChildren)>,
        children: Query<(
            Entity,
            &GlobalTransform,
            Option<&Children>,
            Option<&Handle<Mesh>>,
        )>,
        mut bvhs: ResMut<BvhVec>,
        server: Res<AssetServer>,
    ) {
        for (root, scene) in query.iter() {
            let load_state = server.get_load_state(scene.0.id);
            info!("{:?}", load_state);

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
                    commands
                        .entity(e)
                        .insert(bvhs.add(Bvh::new(&tris)))
                        .insert(Tris(tris))
                        .insert(InvTrans(Mat4::ZERO))
                        .insert(Aabb::default());
                }
            }

            commands
                .entity(root)
                .remove::<BvhInitWithChildren>();
        }
    }

    pub fn update_bvh_data(
        mut query: Query<(&BvhHandle, &GlobalTransform, &mut InvTrans, &mut Aabb)>,
        bvhs: Res<BvhVec>,
    ) {
        for (bvh_handle, trans, mut inv_trans, mut bounds) in query.iter_mut() {
            let trans_matrix = trans.compute_matrix();
            inv_trans.0 = trans_matrix.inverse();

            // calculate world-space bounds using the new matrix
            let root = bvhs.get(bvh_handle).nodes[0];
            let bmin = root.aabb_min;
            let bmax = root.aabb_max;
            for i in 0..8 {
                bounds.grow(trans_matrix.transform_point3(vec3(
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
            &Tris,
            &InvTrans,
            &Aabb,
            &BvhHandle,
        )>,
        bvh_vec: Res<BvhVec>,
        mut raycasts: EventReader<Raycast>,
        mut raycast_results: EventWriter<RaycastResult>,
    ) {
        for raycast in raycasts.iter() {
            let mut target_entity = None;
            let mut ray = raycast.0;
            let mut tmp_distance = ray.t;

            for (e, _trans, tris, inv_trans, bounds, bvh_handle) in query.iter() {
                //if ray.intersect_aabb(bounds.bmin, bounds.bmax) != 1e30f32 {
                let bvh = bvh_vec.get(bvh_handle);
                bvh.intersect(&mut ray, &tris.0, inv_trans);
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

#[derive(Default)]
pub struct BvhVec(Vec<Bvh>);

#[derive(Component, Inspectable)]
pub struct BvhHandle(pub usize);

// TODO: should be something like assets to do this
// Simple add only bvh manager
impl BvhVec {
    pub fn get(&self, id: &BvhHandle) -> &Bvh {
        &self.0[id.0]
    }

    pub fn add(&mut self, bvh: Bvh) -> BvhHandle {
        &self.0.push(bvh);
        BvhHandle(self.0.len() - 1)
    }
}

pub struct Raycast(pub Ray);
pub struct RaycastResult {
    pub entity: Option<Entity>,
    pub world_position: Vec3,
    pub distance: f32,
}

#[derive(Component, Inspectable)]
pub struct Tris(pub Vec<Tri>);

#[derive(Component)]
pub struct InvTrans(pub Mat4);

pub fn save_png(width: u32, height: u32, img: Vec<u8>, filename: impl Into<String>) {
    let mut png_img = RgbImage::new(width, height);
    for (i, pixel_data) in img.chunks(3).enumerate() {
        png_img.put_pixel(
            i as u32 % width,
            i as u32 / width,
            Rgb([pixel_data[0], pixel_data[1], pixel_data[2]]),
        );
    }
    let file = filename.into();
    png_img.save(file.clone()).unwrap();
    println!("Saved {}", file);
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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
