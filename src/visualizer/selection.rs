// See LICENSE file for copyright and license details.

use crate::core::fs::FileSystem;
use crate::core::game_state::GameState;
use crate::core::misc::add_quad_to_vec;
use crate::core::types::UnitId;
use crate::visualizer::geom;
use crate::visualizer::mesh::{Mesh, MeshId};
use crate::visualizer::scene::{Scene, SceneNode, SELECTION_NODE_ID};
use crate::visualizer::shader::Shader;
use crate::visualizer::texture::Texture;
use crate::visualizer::types::{TextureCoord, WorldPos};
use cgmath::Vector2;
use std::path::Path;

pub struct SelectionManager {
    unit_id: Option<UnitId>,
    mesh_id: MeshId,
}

impl SelectionManager {
    pub fn new(mesh_id: MeshId) -> SelectionManager {
        SelectionManager {
            unit_id: None,
            mesh_id,
        }
    }

    fn set_unit_id(&mut self, unit_id: UnitId) {
        self.unit_id = Some(unit_id);
    }

    fn get_pos(&self, state: &GameState) -> WorldPos {
        let unit_id = self.unit_id.unwrap();
        let map_pos = state.units[&unit_id].pos;
        WorldPos {
            v: geom::lift(geom::map_pos_to_world_pos(map_pos).v),
        }
    }

    pub fn move_selection_marker(&self, state: &GameState, scene: &mut Scene) {
        let node = scene.nodes.get_mut(&SELECTION_NODE_ID).unwrap();
        node.pos = self.get_pos(state);
    }

    pub fn create_selection_marker(
        &mut self,
        state: &GameState,
        scene: &mut Scene,
        unit_id: UnitId,
    ) {
        self.set_unit_id(unit_id);
        if scene.nodes.get(&SELECTION_NODE_ID).is_some() {
            let _ = scene.nodes.remove(&SELECTION_NODE_ID).unwrap();
        }
        let node = SceneNode {
            pos: self.get_pos(state),
            rot: 0.0,
            mesh_id: Some(self.mesh_id),
            children: Vec::new(),
        };
        let _ = scene.nodes.insert(SELECTION_NODE_ID, node);
    }

    pub fn deselect(&mut self, scene: &mut Scene) {
        let _ = scene.nodes.remove(&SELECTION_NODE_ID);
        self.unit_id = None;
    }
}

pub fn get_selection_mesh(fs: &FileSystem, shader: &Shader) -> Mesh {
    let tex = Texture::new(&fs.get(&Path::new("data/shell.png")));
    let mut vertex_data = Vec::new();
    let mut tex_data = Vec::new();
    let scale_1 = 0.6;
    let scale_2 = scale_1 + 0.05;
    for num in 0..6 {
        let vertex_1_1 = geom::index_to_hex_vertex_s(scale_1, num);
        let vertex_1_2 = geom::index_to_hex_vertex_s(scale_2, num);
        let vertex_2_1 = geom::index_to_hex_vertex_s(scale_1, num + 1);
        let vertex_2_2 = geom::index_to_hex_vertex_s(scale_2, num + 1);
        add_quad_to_vec(
            &mut vertex_data,
            vertex_2_1,
            vertex_2_2,
            vertex_1_2,
            vertex_1_1,
        );
        add_quad_to_vec(
            &mut tex_data,
            TextureCoord {
                v: Vector2 { x: 0.0, y: 0.0 },
            },
            TextureCoord {
                v: Vector2 { x: 0.0, y: 1.0 },
            },
            TextureCoord {
                v: Vector2 { x: 1.0, y: 1.0 },
            },
            TextureCoord {
                v: Vector2 { x: 1.0, y: 0.0 },
            },
        );
    }
    let mut mesh = Mesh::new(vertex_data.as_slice());
    mesh.set_texture(tex, tex_data.as_slice());
    mesh.prepare(shader);
    mesh
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
