#![allow(warnings)]

mod ray;
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
//use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use image::{Rgb, RgbImage};
use ray::*;
mod aabb;
use aabb::*;
mod tri;
use tri::*;
mod assets;
mod camera;

use bevy::{math::vec3, prelude::*};
use std::mem::swap;

pub mod prelude {
    pub use crate::{aabb::*, assets::*, camera::*, ray::*, tri::*, Bvh, BvhPlugin, BvhInit, Tris};
}

const ROOT_NODE_IDX: usize = 0;
const BINS: usize = 8;

// Markers
#[derive(Component)]
pub struct BvhInit;

pub struct BvhPlugin;
impl Plugin for BvhPlugin {
    fn build(&self, app: &mut App) {
        app
            .register_inspectable::<Bvh>()
            .add_system(spawn_bvh);
    }
}

#[derive(Component)]
pub struct Tris(pub Vec<Tri>);

fn spawn_bvh(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    query: Query<(Entity, &Handle<Mesh>), Added<BvhInit>>,
) {
    for (e, handle) in query.iter() {
        let mesh = meshes.get(handle).expect("Mesh not found");
        let tris = parse_mesh(mesh);
        commands
            .entity(e)
            .insert(Bvh::new(&tris))
            .insert(Tris(tris))
            .remove::<BvhInit>();
    }
}

#[derive(Default, Debug, Clone, Inspectable, Copy)] //
pub struct BvhNode {
    aabb_min: Vec3,
    aabb_max: Vec3,
    left_first: u32,
    tri_count: u32,
}

impl BvhNode {
    pub fn is_leaf(&self) -> bool {
        self.tri_count > 0
    }

    pub fn calculate_cost(&self) -> f32 {
        let e = self.aabb_max - self.aabb_min; // extent of the node
        let surface_area = e.x * e.y + e.y * e.z + e.z * e.x;
        self.tri_count as f32 * surface_area
    }
}

#[derive(Component, Inspectable)]
pub struct Bvh {
    pub nodes: Vec<BvhNode>,
    pub open_node: usize,
    pub triangle_indexs: Vec<usize>,
}

impl Bvh {
    pub fn new(triangles: &[Tri]) -> Bvh {
        let mut bvh = Bvh {
            nodes: vec![BvhNode::default(); triangles.len() * 2],
            open_node: 2,
            triangle_indexs: (0..triangles.len()).collect::<Vec<_>>(),
        };
        let root = &mut bvh.nodes[0];
        root.left_first = 0;
        root.tri_count = triangles.len() as u32;

        bvh.update_node_bounds(0, triangles);
        bvh.subdivide_node(0, triangles);

        bvh
    }

    // pub fn refit(&mut self, triangles: &[Tri]) {
    //     for i in (0..(self.open_node - 1)).rev() {
    //         if i != 1 {
    //             let node = &mut self.nodes[i];
    //             if node.is_leaf() {
    //                 // leaf node: adjust bounds to contained triangles
    //                 self.update_node_bounds(i, triangles);
    //                 continue;
    //             }
    //             // interior node: adjust bounds to child node bounds

    //             let leftChild = &self.nodes[node.left_first as usize];
    //             let rightChild = &self.nodes[(node.left_first + 1) as usize];

    //             node.aabb_min = leftChild.aabb_min.min(rightChild.aabb_min);
    //             node.aabb_max = leftChild.aabb_max.max(rightChild.aabb_max);
    //         }
    //     }
    // }

    fn update_node_bounds(&mut self, node_idx: usize, triangles: &[Tri]) {
        let node = &mut self.nodes[node_idx];
        node.aabb_min = Vec3::splat(1e30f32);
        node.aabb_max = Vec3::splat(-1e30f32);
        for i in 0..node.tri_count {
            let leaf_tri_index = self.triangle_indexs[(node.left_first + i) as usize];
            let leaf_tri = triangles[leaf_tri_index];
            node.aabb_min = node.aabb_min.min(leaf_tri.vertex0);
            node.aabb_min = node.aabb_min.min(leaf_tri.vertex1);
            node.aabb_min = node.aabb_min.min(leaf_tri.vertex2);
            node.aabb_max = node.aabb_max.max(leaf_tri.vertex0);
            node.aabb_max = node.aabb_max.max(leaf_tri.vertex1);
            node.aabb_max = node.aabb_max.max(leaf_tri.vertex2);
        }
    }

    fn subdivide_node(&mut self, node_idx: usize, triangles: &[Tri]) {
        let node = &self.nodes[node_idx];

        // determine split axis using SAH
        let (axis, split_pos, split_cost) = self.find_best_split_plane(node, triangles);
        let nosplit_cost = node.calculate_cost();
        if split_cost >= nosplit_cost {
            return;
        }

        // in-place partition
        let mut i = node.left_first;
        let mut j = i + node.tri_count - 1;
        while i <= j {
            if triangles[self.triangle_indexs[i as usize]].centroid[axis] < split_pos {
                i += 1;
            } else {
                self.triangle_indexs.swap(i as usize, j as usize);
                j -= 1;
            }
        }

        // abort split if one of the sides is empty
        let left_count = i - node.left_first;
        if left_count == 0 || left_count == node.tri_count {
            return;
        }

        // create child nodes
        let left_child_idx = self.open_node as u32;
        self.open_node += 1;
        let right_child_idx = self.open_node as u32;
        self.open_node += 1;

        self.nodes[left_child_idx as usize].left_first = self.nodes[node_idx].left_first;
        self.nodes[left_child_idx as usize].tri_count = left_count;
        self.nodes[right_child_idx as usize].left_first = i;
        self.nodes[right_child_idx as usize].tri_count =
            self.nodes[node_idx].tri_count - left_count;

        self.nodes[node_idx].left_first = left_child_idx;
        self.nodes[node_idx].tri_count = 0;

        self.update_node_bounds(left_child_idx as usize, triangles);
        self.update_node_bounds(right_child_idx as usize, triangles);
        // recurse
        self.subdivide_node(left_child_idx as usize, triangles);
        self.subdivide_node(right_child_idx as usize, triangles);
    }

    pub fn intersect(&self, ray: &mut Ray, triangles: &[Tri]) {
        let mut node = &self.nodes[ROOT_NODE_IDX];
        let mut stack = Vec::with_capacity(64);
        loop {
            if node.is_leaf() {
                for i in 0..node.tri_count {
                    ray.intersect_triangle(
                        &triangles[self.triangle_indexs[(node.left_first + i) as usize]],
                    );
                }
                if stack.is_empty() {
                    break;
                }
                node = stack.pop().unwrap();
                continue;
            }
            let mut child1 = &self.nodes[node.left_first as usize];
            let mut child2 = &self.nodes[(node.left_first + 1) as usize];
            let mut dist1 = ray.intersect_aabb(child1.aabb_min, child1.aabb_max);
            let mut dist2 = ray.intersect_aabb(child2.aabb_min, child2.aabb_max);
            if dist1 > dist2 {
                swap(&mut dist1, &mut dist2);
                swap(&mut child1, &mut child2);
            }
            if dist1 == 1e30f32 {
                if stack.is_empty() {
                    break;
                }
                node = stack.pop().unwrap();
            } else {
                node = child1;
                if dist2 != 1e30f32 {
                    stack.push(child2);
                }
            }
        }
    }

    fn find_best_split_plane(&self, node: &BvhNode, triangles: &[Tri]) -> (usize, f32, f32) {
        // determine split axis using SAH
        let mut best_axis = 0;
        let mut split_pos = 0.0f32;
        let mut best_cost = 1e30f32;

        for a in 0..3 {
            let mut bounds_min = 1e30f32;
            let mut bounds_max = -1e30f32;
            for i in 0..node.tri_count {
                let triangle = &triangles[self.triangle_indexs[(node.left_first + i) as usize]];
                bounds_min = bounds_min.min(triangle.centroid[a]);
                bounds_max = bounds_max.max(triangle.centroid[a]);
            }
            if bounds_min == bounds_max {
                continue;
            }
            // populate bins
            let mut bin = [Bin::default(); BINS];
            let mut scale = BINS as f32 / (bounds_max - bounds_min);
            for i in 0..node.tri_count {
                let triangle = &triangles[self.triangle_indexs[(node.left_first + i) as usize]];
                let bin_idx =
                    (BINS - 1).min(((triangle.centroid[a] - bounds_min) * scale) as usize);
                bin[bin_idx].tri_count += 1;
                bin[bin_idx].bounds.grow(triangle.vertex0);
                bin[bin_idx].bounds.grow(triangle.vertex1);
                bin[bin_idx].bounds.grow(triangle.vertex2);
            }

            // gather data for the BINS - 1 planes between the bins
            let mut left_area = [0.0f32; BINS - 1];
            let mut right_area = [0.0f32; BINS - 1];
            let mut left_count = [0u32; BINS - 1];
            let mut right_count = [0u32; BINS - 1];
            let mut left_box = Aabb::default();
            let mut right_box = Aabb::default();
            let mut left_sum = 0u32;
            let mut right_sum = 0u32;
            for i in 0..(BINS - 1) {
                left_sum += bin[i].tri_count;
                left_count[i] = left_sum;
                left_box.grow_aabb(&bin[i].bounds);
                left_area[i] = left_box.area();
                right_sum += bin[BINS - 1 - i].tri_count;
                right_count[BINS - 2 - i] = right_sum;
                right_box.grow_aabb(&bin[BINS - 1 - i].bounds);
                right_area[BINS - 2 - i] = right_box.area();
            }

            // calculate SAH cost for the 7 planes
            scale = (bounds_max - bounds_min) / BINS as f32;
            for i in 0..BINS - 1 {
                let plane_cost =
                    left_count[i] as f32 * left_area[i] + right_count[i] as f32 * right_area[i];
                if plane_cost < best_cost {
                    best_axis = a;
                    split_pos = bounds_min + scale * (i + 1) as f32;
                    best_cost = plane_cost;
                }
            }
        }
        (best_axis, split_pos, best_cost)
    }
}

#[derive(Default, Debug, Copy, Clone)]
struct Bin {
    bounds: Aabb,
    tri_count: u32,
}

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
                bevy::render::mesh::VertexAttributeValues::Float32x3(vec) => vec,
                _ => todo!(),
            };

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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
