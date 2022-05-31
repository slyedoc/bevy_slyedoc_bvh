use std::mem::swap;

use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;

use crate::{prelude::Ray, Bvh, BvhInstance};

#[derive(Default, Inspectable, Copy, Clone)]
pub struct TlasNode {
    pub aabb_min: Vec3,
    pub left_right: u32, // 2x16 bits
    pub aabb_max: Vec3,
    pub blas: u32,
}

impl TlasNode {
    pub fn is_leaf(&self) -> bool {
        self.left_right == 0
    }
}

#[derive(Inspectable)]
pub struct Tlas {
    pub tlas_nodes: Vec<TlasNode>,
    pub blas: Vec<BvhInstance>,
    pub bvhs: Vec<Bvh>,
}

impl Default for Tlas {
    fn default() -> Self {
        Tlas {
            tlas_nodes: Vec::with_capacity(0),
            blas: Default::default(),
            bvhs: Default::default(),
        }
    }
}

impl Tlas {
    pub fn add_bvh(&mut self, bvh: Bvh) -> usize {
        self.bvhs.push(bvh);
        self.bvhs.len() - 1
    }

    pub fn add_instance(&mut self, instnace: BvhInstance) {
        self.blas.push(instnace);
    }

    pub fn build(&mut self) {
        self.tlas_nodes = Vec::with_capacity(self.blas.len() + 1);
        // reserve root node
        self.tlas_nodes.push(TlasNode::default());

        let mut node_index = vec![0u32; self.blas.len()+1]; 
        let mut node_indices = self.blas.len() as i32;
        // assign a TLASleaf node to each BLAS

        for (i, b) in self.blas.iter().enumerate() {   
            node_index[i] = i as u32 + 1;         
            self.tlas_nodes.push(TlasNode {
                aabb_min: b.bounds.bmin,
                aabb_max: b.bounds.bmax,
                left_right: 0, // leaf
                blas: i as u32,
            });
        }

        // use agglomerative clustering to build the TLAS
        let mut a = 0i32;
        let mut b = self.find_best_match(&node_index, node_indices, a);
        while node_indices > 1
         {
         	let c = self.find_best_match( &node_index, node_indices, b);
         	if a == c {
         		let mut nodeIdxA = node_index[a as usize];
                let mut nodeIdxB = node_index[b as usize];
         		let nodeA = &self.tlas_nodes[nodeIdxA as usize];
         		let nodeB = &self.tlas_nodes[nodeIdxB as usize];
         		let newNode = &self.tlas_nodes.push(TlasNode {
                    aabb_min: nodeA.aabb_min.min( nodeB.aabb_min ),
                    aabb_max: nodeA.aabb_max.max( nodeB.aabb_max),
                    left_right: nodeIdxA + (nodeIdxB << 16),                    
                    blas: 0,
                });
                node_index[a as usize] = self.tlas_nodes.len() as u32 - 1;
                node_index[b as usize] = node_index[node_indices as usize - 1];
                node_indices -= 1;
         		b = self.find_best_match(&node_index, node_indices, a );
         	} else {
                a = b;
                b = c;
             } 
         }
        self.tlas_nodes[0] = self.tlas_nodes[node_index[a as usize] as usize];
    }

    pub fn find_best_match(&self, list: &[u32], N: i32, a: i32) -> i32 {
        let mut smallest = 1e30f32;
        let mut best_b = -1i32;
        for b in 0..N {
            if b != a {
                let bmax = self.tlas_nodes[list[a as usize] as usize]
                    .aabb_max
                    .max(self.tlas_nodes[list[b as usize] as usize].aabb_max);
                let bmin = self.tlas_nodes[list[a as usize] as usize]
                    .aabb_min
                    .min(self.tlas_nodes[list[b as usize] as usize].aabb_min);
                let e = bmax - bmin;
                let surfaceArea = e.x * e.y + e.y * e.z + e.z * e.x;
                if surfaceArea < smallest {
                    smallest = surfaceArea;
                    best_b = b;
                }
            }
        }
        return best_b;
    }

    pub fn update_bvh(&mut self, query: &Query<(&GlobalTransform)>) {
        for mut instance in &mut self.blas {
            let bvh = &self.bvhs[instance.bvh_index];
            if let Some(e) = instance.entity && let Ok(trans) = query.get(e) {
                instance.update(trans, &bvh.nodes[0]);
            }
        }
    }
}
