// See LICENSE file for copyright and license details.

use core::core::{UnitTypeId};
use visualizer::types::{MFloat};
use visualizer::mesh::{MeshId};

pub struct UnitTypeVisualInfo {
    pub mesh_id: MeshId,
    pub move_speed: MFloat, // TODO: MFloat -> Speed
}

pub struct UnitTypeVisualInfoManager {
    list: Vec<UnitTypeVisualInfo>,
}

impl UnitTypeVisualInfoManager {
    pub fn new() -> UnitTypeVisualInfoManager {
        UnitTypeVisualInfoManager {
            list: vec![],
        }
    }

    pub fn add_info(&mut self, info: UnitTypeVisualInfo) {
        self.list.push(info);
    }

    pub fn get<'a>(&'a self, type_id: UnitTypeId) -> &'a UnitTypeVisualInfo {
        &self.list[type_id.id as uint]
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
