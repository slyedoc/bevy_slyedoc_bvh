use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;

use crate::{Bvh, prelude::Ray, BvhInstance};

#[derive(Default, Inspectable)]
pub struct TlasNode {
    aabb_min: Vec3,
    left_blas: u32,
    aabb_max: Vec3,
    is_leaf: bool,
}

#[derive(Default, Inspectable)]
pub struct Tlas {
    pub nodes: Vec<TlasNode>,
    pub blas: Vec<BvhInstance>,
    nodes_used: u32,
    blas_count: u32,
}

impl Tlas {
    pub fn build(&mut self) {
        // assign a TLASleaf node to each BLAS
        let n2 = TlasNode {
            left_blas: 0,
            aabb_min: Vec3::splat(-100.0),
            aabb_max: Vec3::splat(100.0),
            is_leaf: true,
        };

        let n3 = TlasNode {
            left_blas: 1,
            aabb_min: Vec3::splat(-100.0),
            aabb_max: Vec3::splat(100.0),
            is_leaf: true,
        };

        let root = TlasNode {
            left_blas: 2,
            aabb_min: Vec3::splat(-100.0),
            aabb_max: Vec3::splat(100.0),
            is_leaf: true,
        };
        self.nodes.extend([root, TlasNode::default(), n2, n3,]);
    }
    
    pub fn intersect(&self, ray: &mut Ray, bvhs: Assets<Bvh> ) {

        let mut node =  &self.nodes[0];
        let mut stack = Vec::<usize>::with_capacity(64);
        loop {
            if node.is_leaf {

                self.blas[node.left_blas as usize].intersect( ray, &bvhs);
                if let Some(n) = stack.pop() {
                    node = &self.nodes[n];
                    continue;
                } else {
                    break;
                }
                continue;
            }
            let child1 = &self.nodes[node.left_blas as usize];
            let child2 = &self.nodes[node.left_blas as usize + 1];
            let dist1 = ray.intersect_aabb(child1.aabb_min, child1.aabb_max );
            let dist2 = ray.intersect_aabb(child2.aabb_min, child2.aabb_max );
            // if (dist1 > dist2) { swap( dist1, dist2 ); swap( child1, child2 ); }
            // if (dist1 == 1e30f)
            // {
            //     if (stackPtr == 0) break; else node = stack[--stackPtr];
            // }
            // else
            // {
            //     node = child1;
            //     if (dist2 != 1e30f) stack[stackPtr++] = child2;
            // }
        }
    }

}
