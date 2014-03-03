// See LICENSE file for copyright and license details.

use collections::hashmap::HashMap;
use serialize::{Decodable, json};
use glfw;
use cgmath::vector::{Vec3, Vec2};
use core::map::MapPosIter;
use core::types::{Size2, MInt, MBool, UnitId, PlayerId, MapPos};
use core::game_state::GameState;
use core::pathfinder::Pathfinder;
use core::misc::read_file;
use core::core;
use visualizer::gl_helpers::{
    uniform_mat4f,
    set_clear_color,
    clear_screen,
    init_opengl,
    load_gl_funcs_with,
    set_viewport,
    tr,
};
use visualizer::camera::Camera;
use visualizer::geom::Geom;
use visualizer::tile_picker::TilePicker;
use visualizer::obj;
use visualizer::mesh::Mesh;
use visualizer::types::{
    Scene,
    VertexCoord,
    TextureCoord,
    MFloat,
    Point2,
    MatId,
};
use visualizer::event_visualizer::{
    EventVisualizer,
    EventMoveVisualizer,
    EventEndTurnVisualizer,
    EventCreateUnitVisualizer,
    EventAttackUnitVisualizer,
};
use visualizer::shader::Shader;
use visualizer::texture::Texture;

fn build_hex_mesh(&geom: &Geom, map_size: Size2<MInt>) -> ~[VertexCoord] {
    let mut vertex_data = ~[];
    for tile_pos in MapPosIter::new(map_size) {
        let pos = geom.map_pos_to_world_pos(tile_pos);
        for num in range(0 as MInt, 6) {
            let vertex = geom.index_to_hex_vertex(num);
            let next_vertex = geom.index_to_hex_vertex(num + 1);
            vertex_data.push(pos + vertex);
            vertex_data.push(pos + next_vertex);
            vertex_data.push(pos + Vec3::zero());
        }
    }
    vertex_data
}

fn build_hex_tex_coord(map_size: Size2<MInt>) -> ~[TextureCoord] {
    let mut vertex_data = ~[];
    for _ in MapPosIter::new(map_size) {
        for _ in range(0, 6) {
            vertex_data.push(Vec2{x: 0.0, y: 0.0});
            vertex_data.push(Vec2{x: 1.0, y: 0.0});
            vertex_data.push(Vec2{x: 0.5, y: 0.5});
        }
    }
    vertex_data
}

// TODO: Simplify
fn read_win_size(config_path: &str) -> Size2<MInt> {
    let path = Path::new(config_path);
    let mut json = match json::from_str(read_file(&path)) {
        Ok(json::Object(obj)) => obj,
        _ => fail!("Config error"),
    };
    let screen_size = match json.pop(&~"screen_size") {
        Some(size) => size,
        None => fail!("No field 'screen_size'"),
    };
    let extructed_size: Size2<MInt> = Decodable::decode(
        &mut json::Decoder::new(screen_size));
    extructed_size
}

fn init_win(win_size: Size2<MInt>) -> glfw::Window {
    glfw::set_error_callback(~glfw::LogErrorHandler);
    let init_status = glfw::init();
    if !init_status.is_ok() {
        fail!("Failed to initialize GLFW");
    }
    let win = glfw::Window::create(
        win_size.w as u32,
        win_size.h as u32,
        "Marauder",
        glfw::Windowed,
    ).unwrap();
    win.make_context_current();
    win.set_all_polling(true);
    win
}

fn get_scenes(players_count: MInt) -> HashMap<PlayerId, Scene> {
    let mut m = HashMap::new();
    for i in range(0, players_count) {
        m.insert(PlayerId(i), HashMap::new());
    }
    m
}

fn get_game_states(players_count: MInt) -> HashMap<PlayerId, GameState> {
    let mut m = HashMap::new();
    for i in range(0, players_count) {
        m.insert(PlayerId(i), GameState::new());
    }
    m
}

fn get_pathfinders(
    players_count: MInt,
    map_size: Size2<MInt>,
) -> HashMap<PlayerId, Pathfinder> {
    let mut m = HashMap::new();
    for i in range(0, players_count) {
        m.insert(PlayerId(i), Pathfinder::new(map_size));
    }
    m
}

fn get_map_mesh(geom: &Geom, map_size: Size2<MInt>, shader: &Shader) -> Mesh {
    let tex = Texture::new(~"data/floor.png");
    let mut mesh = Mesh::new(build_hex_mesh(geom, map_size));
    mesh.set_texture(tex, build_hex_tex_coord(map_size));
    mesh.prepare(shader);
    mesh
}

fn load_unit_mesh(shader: &Shader) -> Mesh {
    let tex = Texture::new(~"data/tank.png");
    let obj = obj::Model::new("data/tank.obj");
    let mut mesh = Mesh::new(obj.build());
    mesh.set_texture(tex, obj.build_tex_coord());
    mesh.prepare(shader);
    mesh
}

pub struct Visualizer<'a> {
    shader: Shader,
    map_mesh: Mesh,
    unit_mesh: Mesh,
    mvp_mat_id: MatId,
    win: glfw::Window,
    mouse_pos: Point2<MFloat>,
    camera: Camera,
    tile_picker: ~TilePicker,
    selected_tile_pos: Option<MapPos>,
    selected_unit_id: Option<UnitId>,
    geom: Geom,
    scenes: HashMap<PlayerId, Scene>,
    core: ~core::Core<'a>,
    event: Option<core::Event>,
    event_visualizer: Option<~EventVisualizer>,
    game_state: HashMap<PlayerId, GameState>,
    pathfinders: HashMap<PlayerId, Pathfinder>,
}

impl<'a> Visualizer<'a> {
    pub fn new() -> ~Visualizer {
        let players_count = 2;
        let win_size = read_win_size("config.json");
        let win = init_win(win_size);
        load_gl_funcs_with(glfw::get_proc_address);
        init_opengl();
        let geom = Geom::new();
        let core = core::Core::new();
        let map_size = core.map_size();
        let tile_picker = TilePicker::new(
            win_size, &geom, core.map_size());
        let shader = Shader::new("normal.vs.glsl", "normal.fs.glsl");
        let mvp_mat_id = MatId(shader.get_uniform("mvp_mat"));
        let vis = ~Visualizer {
            map_mesh: get_map_mesh(&geom, map_size, &shader),
            unit_mesh: load_unit_mesh(&shader),
            mvp_mat_id: mvp_mat_id,
            shader: shader,
            win: win,
            mouse_pos: Vec2::zero(),
            camera: Camera::new(),
            tile_picker: tile_picker,
            selected_tile_pos: None,
            selected_unit_id: None,
            geom: geom,
            core: core,
            event_visualizer: None,
            event: None,
            scenes: get_scenes(players_count),
            game_state: get_game_states(players_count),
            pathfinders: get_pathfinders(players_count, map_size),
        };
        vis
    }

    fn win<'a>(&'a self) -> &'a glfw::Window {
        &self.win
    }

    fn scene<'a>(&'a self) -> &'a Scene {
        self.scenes.get(&self.core.player_id())
    }

    fn unit_at_opt(&'a self, pos: MapPos) -> Option<&'a core::Unit> {
        let mut res = None;
        let id = self.core.player_id();
        for (_, unit) in self.game_state.get(&id).units.iter() {
            if unit.pos == pos {
                res = Some(unit);
                break;
            }
        }
        res
    }

    fn draw_units(&self) {
        for (_, unit) in self.scene().iter() {
            let m = tr(self.camera.mat(), unit.pos);
            uniform_mat4f(self.mvp_mat_id, &m);
            self.unit_mesh.draw(&self.shader);
        }
    }

    fn draw_map(&self) {
        uniform_mat4f(self.mvp_mat_id, &self.camera.mat());
        self.map_mesh.draw(&self.shader);
    }

    fn draw(&mut self) {
        set_clear_color(0.3, 0.3, 0.3);
        clear_screen();
        self.shader.activate();
        self.draw_units();
        self.draw_map();
        if !self.event_visualizer.is_none() {
            let scene = self.scenes.get_mut(&self.core.player_id());
            self.event_visualizer.get_mut_ref().draw(&self.geom, scene);
        }
        self.win().swap_buffers();
    }

    pub fn is_running(&self) -> MBool {
        return !self.win().should_close()
    }

    fn end_turn(&mut self) {
        self.core.do_command(core::CommandEndTurn);
        self.selected_unit_id = None;
    }

    fn create_unit(&mut self) {
        let pos_opt = self.selected_tile_pos;
        if pos_opt.is_some() {
            let pos = pos_opt.unwrap();
            if self.unit_at_opt(pos).is_none() {
                let cmd = core::CommandCreateUnit(pos);
                self.core.do_command(cmd);
            }
        }
    }

    fn attack_unit(&mut self) {
        let pos_opt = self.selected_tile_pos;
        if pos_opt.is_some() {
            let pos = pos_opt.unwrap();
            if self.unit_at_opt(pos).is_some() {
                let defender_id = self.unit_at_opt(pos).unwrap().id;
                let cmd = core::CommandAttackUnit(UnitId(0), defender_id);
                self.core.do_command(cmd);
            }
        }
    }

    fn handle_key_event(&mut self, key: glfw::Key) {
        match key {
            glfw::KeyEscape | glfw::KeyQ => {
                self.win().set_should_close(true);
            },
            glfw::KeyUp => self.camera.move(270.0),
            glfw::KeyDown => self.camera.move(90.0),
            glfw::KeyRight => self.camera.move(0.0),
            glfw::KeyLeft => self.camera.move(180.0),
            glfw::KeyMinus => self.camera.zoom += 1.0,
            glfw::KeyEqual => self.camera.zoom -= 1.0,
            _ => {},
        }
        if self.event_visualizer.is_some() {
            return;
        }
        match key {
            glfw::KeyT => self.end_turn(),
            glfw::KeyU => self.create_unit(),
            glfw::KeyA => self.attack_unit(),
            _ => {},
        }
    }

    fn handle_cursor_pos_event(&mut self, pos: Point2<MFloat>) {
        let button = self.win().get_mouse_button(glfw::MouseButtonRight);
        if button == glfw::Press {
            self.camera.z_angle += (self.mouse_pos.x - pos.x) / 2.0;
            self.camera.x_angle += (self.mouse_pos.y - pos.y) / 2.0;
        }
        self.mouse_pos = pos;
    }

    fn handle_mouse_button_event(&mut self) {
        if self.event_visualizer.is_some() {
            return;
        }
        if self.selected_tile_pos.is_some() {
            let pos = self.selected_tile_pos.unwrap();
            if self.unit_at_opt(pos).is_some() {
                let unit_id = self.unit_at_opt(pos).unwrap().id;
                let state = self.game_state.get_mut(&self.core.player_id());
                self.selected_unit_id = Some(unit_id);
                let pf = self.pathfinders.get_mut(&self.core.player_id());
                pf.fill_map(state, state.units.get(&unit_id));
            } else if self.selected_unit_id.is_some() {
                let unit_id = self.selected_unit_id.unwrap();
                let pf = self.pathfinders.get_mut(&self.core.player_id());
                let path = pf.get_path(pos);
                self.core.do_command(core::CommandMove(unit_id, path));
            }
        }
    }

    fn get_events(&mut self) -> ~[glfw::WindowEvent] {
        glfw::poll_events();
        let mut events = ~[];
        for (_, event) in self.win().flush_events() {
            events.push(event);
        }
        events
    }

    fn handle_event(&mut self, event: glfw::WindowEvent) {
        match event {
            glfw::KeyEvent(key, _, glfw::Press, _) => {
                self.handle_key_event(key);
            },
            glfw::CursorPosEvent(x, y) => {
                let p = Vec2{x: x as MFloat, y: y as MFloat};
                self.handle_cursor_pos_event(p);
            },
            glfw::MouseButtonEvent(glfw::MouseButtonLeft, glfw::Press, _) => {
                self.handle_mouse_button_event();
            },
            glfw::SizeEvent(w, h) => {
                set_viewport(Size2{w: w, h: h});
                self.tile_picker.set_win_size(Size2{w: w, h: h});
            },
            _ => {},
        }
    }

    fn handle_events(&mut self) {
        for event in self.get_events().iter() {
            self.handle_event(*event);
        }
    }

    fn pick_tile(&mut self) {
        let mouse_pos = Vec2 {
            x: self.mouse_pos.x as MInt,
            y: self.mouse_pos.y as MInt,
        };
        self.selected_tile_pos =
            self.tile_picker.pick_tile(&self.camera, mouse_pos);
    }

    fn make_event_visualizer(&mut self, event: &core::Event) -> ~EventVisualizer {
        match *event {
            core::EventMove(ref unit_id, ref path) => {
                EventMoveVisualizer::new(*unit_id, path.clone())
            },
            core::EventEndTurn(_, _) => {
                EventEndTurnVisualizer::new()
            },
            core::EventCreateUnit(id, ref pos) => {
                EventCreateUnitVisualizer::new(id, *pos)
            },
            core::EventAttackUnit(attacker_id, defender_id) => {
                EventAttackUnitVisualizer::new(attacker_id, defender_id)
            },
        }
    }

    fn logic(&mut self) {
        if self.event_visualizer.is_none() {
            let event_opt = self.core.get_event();
            if event_opt.is_some() {
                let event = event_opt.unwrap();
                let vis = self.make_event_visualizer(&event);
                self.event = Some(event);
                self.event_visualizer = Some(vis);
                let scene = self.scenes.get_mut(&self.core.player_id());
                self.event_visualizer.get_mut_ref().start(&self.geom, scene);
            }
        } else if self.event_visualizer.get_ref().is_finished() {
            let scene = self.scenes.get_mut(&self.core.player_id());
            self.event_visualizer.get_mut_ref().end(&self.geom, scene);
            let state = self.game_state.get_mut(&self.core.player_id());
            state.apply_event(self.event.get_ref());
            self.event_visualizer = None;
            self.event = None;
            if self.selected_unit_id.is_some() {
                let unit_id = self.selected_unit_id.unwrap();
                let pf = self.pathfinders.get_mut(&self.core.player_id());
                pf.fill_map(state, state.units.get(&unit_id));
            }
        }
    }

    pub fn tick(&mut self) {
        self.handle_events();
        self.logic();
        self.pick_tile();
        self.draw();
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab: