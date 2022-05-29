use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;
use crate::prelude::*;

#[derive(Component, Inspectable, Debug)]
pub struct Bvh {
    pub nodes: Vec<BvhNode>,
    pub triangle_indexs: Vec<usize>,
}

impl Bvh {
    // TODO: need far better way to get tris from bevy mesh
    pub fn new(triangles: &[Tri]) -> Bvh {
        let mut bvh = Bvh {
            nodes:  {
                // Add root node and empty node to offset 1
                let mut nodes = Vec::with_capacity(64);
                nodes.push(BvhNode {
                    left_first: 0,
                    tri_count: triangles.len() as u32,
                    aabb_min: Vec3::ZERO,
                    aabb_max: Vec3::ZERO,
                });
                nodes.push(BvhNode {
                    left_first: 0,
                    tri_count: 0,
                    aabb_min: Vec3::ZERO,
                    aabb_max: Vec3::ZERO,
                });
                nodes
            },
            triangle_indexs: (0..triangles.len()).collect::<Vec<_>>(),
        };

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
        self.nodes.push(BvhNode::default());
        let left_child_idx =  self.nodes.len() as u32 - 1;
        self.nodes.push(BvhNode::default());
        let right_child_idx = self.nodes.len() as u32 - 1;

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

    pub fn intersect(&self, ray: &mut Ray, triangles: &[Tri], inv_trans: &InvTrans) {
        // backup ray and transform original
        let mut backupRay = ray.clone();

        ray.origin = inv_trans.0.transform_point3(ray.origin);
        ray.direction = inv_trans.0.transform_vector3(ray.direction);
        ray.direction_inv = ray.direction.recip();

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

        // restore ray origin and direction
        backupRay.t = ray.t;
        *ray = backupRay;
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

#[derive(Default, Debug, Clone, Inspectable, Copy)]
pub struct BvhNode {
    pub aabb_min: Vec3,
    pub aabb_max: Vec3,
    pub left_first: u32,
    pub tri_count: u32,
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

#[derive(Default, Debug, Copy, Clone)]
struct Bin {
    bounds: Aabb,
    tri_count: u32,
}

