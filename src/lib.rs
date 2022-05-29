#![allow(warnings)]
use bevy_inspector_egui::{
    plugin::InspectorWindows, Inspectable, InspectorPlugin, RegisterInspectable,
};
mod ray;
use ray::*;
mod aabb;
use aabb::*;
mod tri;
use tri::*;
mod assets;
mod bvh;
use bvh::*;
mod camera;
use bevy::{
    asset::LoadState,
    math::{vec2, vec3},
    prelude::*,
    reflect::TypeUuid,
    render::{
        camera::{Camera3d, CameraProjection},
        render_resource::{Extent3d, Texture, TextureDimension, TextureFormat},
    },
    transform::TransformSystem, utils::Instant,
};
use camera::*;
use std::mem::swap;

pub mod prelude {
    pub use crate::{aabb::*, assets::*, bvh::*, camera::*, ray::*, tri::*, *};
}

const ROOT_NODE_IDX: usize = 0;
const BINS: usize = 8;

pub struct BvhPlugin;
impl Plugin for BvhPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BvhImage>()
            .add_event::<Raycast>()
            .add_event::<RaycastResult>()
            //.add_plugin(InspectorPlugin::<BvhImage>::new())
            .init_resource::<BvhImage>()
            .init_resource::<BvhVec>()
            .init_resource::<BvhStats>()
            .register_inspectable::<Bvh>()
            .register_inspectable::<BvhCamera>()
            .register_inspectable::<BvhHandle>()
            .register_inspectable::<Tris>()
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
            .add_system_set_to_stage(
                CoreStage::PostUpdate,
                SystemSet::new()
                    .with_system(Self::update_camera)
                    .with_system(Self::render_camera.after(Self::update_camera)),
            );
    }
}

#[derive(Default)]
pub struct BvhStats {
    pub tri_count: usize,
}

impl BvhPlugin {
    fn spawn_bvh(
        mut commands: Commands,
        meshes: Res<Assets<Mesh>>,
        query: Query<(Entity, &Handle<Mesh>), With<BvhInit>>,
        mut bvhs: ResMut<BvhVec>,
        server: Res<AssetServer>,
        mut stats: ResMut<BvhStats>,
    ) {
        for (e, handle) in query.iter() {
            let loaded = server.get_load_state(handle.id);
            // info!("loaded {:?}", loaded);
            // if loaded != LoadState::Loaded {
            //     continue;
            // }

            let mesh = meshes.get(handle).expect("Mesh not found");
            let tris = parse_mesh(mesh);
            stats.tri_count += tris.len();
            commands
                .entity(e)
                .insert(bvhs.add(Bvh::new(&tris)))
                .insert(Tris(tris))
                .insert(InvTrans(Mat4::ZERO))
                .insert(Aabb::default())
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
        mut bvhs: ResMut<BvhVec>,
        server: Res<AssetServer>,
        mut stats: ResMut<BvhStats>,
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
                        .insert(bvhs.add(Bvh::new(&tris)))
                        .insert(Tris(tris))
                        .insert(InvTrans(Mat4::ZERO))
                        .insert(Aabb::default());
                }
            }

            commands.entity(root).remove::<BvhInitWithChildren>();
        }
    }

    pub fn update_bvh_data(
        mut query: Query<(&BvhHandle, &GlobalTransform, &mut InvTrans, &mut Aabb)>,
        bvhs: Res<BvhVec>,
    ) {
        for (bvh_handle, trans, mut inv_trans, mut bounds) in query.iter_mut() {
            // Update inv transfrom matrix for faster intersections
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

    pub fn update_camera(mut camera_query: Query<(&mut BvhCamera, &GlobalTransform)>) {
        for (mut camera, trans) in camera_query.iter_mut() {
            camera.update(trans);
        }
    }

    pub fn render_camera(
        query: Query<(
            Entity,
            &GlobalTransform,
            &Tris,
            &InvTrans,
            &Aabb,
            &BvhHandle,
        )>,
        camera_query: Query<(&BvhCamera)>,
        mut bvh_image: ResMut<BvhImage>,
        bvh_vec: Res<BvhVec>,
        mut images: ResMut<Assets<Image>>,
        mut keys: ResMut<Input<KeyCode>>,
    ) {
        if keys.just_pressed(KeyCode::Space) {
            let start = Instant::now();
            let mut image = images.get_mut(bvh_image.image.clone()).unwrap();
            let (camera) = camera_query.single();

            for i in 0..(bvh_image.height * bvh_image.width) {
                let x = i % bvh_image.width;
                let y = i / bvh_image.width;

                let u = x as f32 / bvh_image.width as f32;
                let v = y as f32 / bvh_image.height as f32;
                let mut ray = camera.get_ray(u, v);

                let mut t = ray.t;
                let mut target_entity = None;
                for (e, _trans, tris, inv_trans, bounds, bvh_handle) in query.iter() {
                    //if ray.intersect_aabb(bounds.bmin, bounds.bmax) != 1e30f32 {
                    let bvh = bvh_vec.get(bvh_handle);
                    bvh.intersect(&mut ray, &tris.0, &inv_trans);
                    if t != ray.t {
                        target_entity = Some((e, ray));
                        t = ray.t;
                    }
                }

                let pixel_index = ((bvh_image.height - y - 1) * bvh_image.width + x) as usize * 4;
                if let Some((e, ray)) = target_entity {
                    let c = 900f32 - (ray.t * 42f32);
                    let c = c as u8;
                    image.data[pixel_index + 0] = c;
                    image.data[pixel_index + 1] = c;
                    image.data[pixel_index + 2] = c;
                    image.data[pixel_index + 3] = 255;
                } else {
                    image.data[pixel_index + 0] = 0;
                    image.data[pixel_index + 1] = 0;
                    image.data[pixel_index + 2] = 0;
                    image.data[pixel_index + 3] = 255;
                }
            }
            info!("Render time: {:?}", start.elapsed());
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

// pub fn save_png(width: u32, height: u32, img: Vec<u8>, filename: impl Into<String>) {
//     let mut png_img = RgbImage::new(width, height);
//     for (i, pixel_data) in img.chunks(3).enumerate() {
//         png_img.put_pixel(
//             i as u32 % width,
//             i as u32 / width,
//             Rgb([pixel_data[0], pixel_data[1], pixel_data[2]]),
//         );
//     }
//     let file = filename.into();
//     png_img.save(file.clone()).unwrap();
//     println!("Saved {}", file);
// }

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
