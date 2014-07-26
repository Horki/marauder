// See LICENSE file for copyright and license details.

#![macro_escape]

use std;
use gl;
use gl::types::{GLuint, GLsizeiptr};
use cgmath::matrix::{Matrix, Matrix4, Matrix3, ToMatrix4};
use cgmath::array::Array2;
use cgmath::vector::{Vector3};
use cgmath::angle;
use cgmath::projection;
use core::misc::deg_to_rad;
use core::types::{Size2, MInt};
use visualizer::types::{MFloat, Color3, Color4, ScreenPos};

pub use load_gl_funcs_with = gl::load_with;

pub static GREY_3: Color3 = Color3{r: 0.3, g: 0.3, b: 0.3};
pub static BLACK_3: Color3 = Color3{r: 0.0, g: 0.0, b: 0.0};
pub static WHITE: Color4 = Color4{r: 1.0, g: 1.0, b: 1.0, a: 1.0};
pub static BLUE: Color4 = Color4{r: 0.0, g: 0.0, b: 1.0, a: 1.0};
pub static BLACK: Color4 = Color4{r: 0.0, g: 0.0, b: 0.0, a: 1.0};

macro_rules! verify(
    ($e: expr) => ({
        let result = $e;
        let error_code = gl::GetError();
        if error_code != 0 {
            let description = match error_code {
                gl::INVALID_ENUM =>                  "GL_INVALID_ENUM",
                gl::INVALID_FRAMEBUFFER_OPERATION => "GL_INVALID_FRAMEBUFFER_OPERATION",
                gl::INVALID_OPERATION =>             "GL_INVALID_OPERATION",
                gl::INVALID_VALUE =>                 "GL_INVALID_VALUE",
                gl::NO_ERROR =>                      "GL_NO_ERROR",
                gl::OUT_OF_MEMORY =>                 "GL_OUT_OF_MEMORY",
                _ => fail!("Bad gl error code: {}", error_code),
            };
            fail!("gl error: {}({})", description, error_code);
        }
        result
    })
)

pub enum MeshRenderMode {
    Triangles,
    Lines,
}

impl MeshRenderMode {
    fn to_gl_type(&self) -> GLuint {
        match *self {
            Triangles => gl::TRIANGLES,
            Lines => gl::LINES,
        }
    }
}

pub fn tr(m: Matrix4<MFloat>, v: Vector3<MFloat>) -> Matrix4<MFloat> {
    let mut t = Matrix4::<MFloat>::identity();
    *t.mut_cr(3, 0) = v.x;
    *t.mut_cr(3, 1) = v.y;
    *t.mut_cr(3, 2) = v.z;
    m.mul_m(&t)
}

pub fn scale(m: Matrix4<MFloat>, scale: MFloat) -> Matrix4<MFloat> {
    let mut t = Matrix4::<MFloat>::identity();
    *t.mut_cr(0, 0) = scale;
    *t.mut_cr(1, 1) = scale;
    *t.mut_cr(2, 2) = scale;
    m.mul_m(&t)
}

pub fn rot_x(m: Matrix4<MFloat>, angle: MFloat) -> Matrix4<MFloat> {
    let rad = angle::rad(deg_to_rad(angle));
    let r = Matrix3::from_angle_x(rad).to_matrix4();
    m.mul_m(&r)
}

pub fn rot_z(m: Matrix4<MFloat>, angle: MFloat) -> Matrix4<MFloat> {
    let rad = angle::rad(deg_to_rad(angle));
    let r = Matrix3::from_angle_z(rad).to_matrix4();
    m.mul_m(&r)
}

pub fn init_opengl() {
    verify!(gl::Enable(gl::DEPTH_TEST));
    verify!(gl::Enable(gl::BLEND));
    verify!(gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA));
}

pub fn set_clear_color(color: Color3) {
    verify!(gl::ClearColor(color.r, color.g, color.b, 1.0));
}

pub fn clear_screen() {
    verify!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));
}

pub fn set_viewport(size: Size2<MInt>) {
    verify!(gl::Viewport(0, 0, size.w, size.h));
}

pub struct Vao {
    id: GLuint,
}

impl Vao {
    pub fn new() -> Vao {
        let mut id = 0;
        unsafe {
            verify!(gl::GenVertexArrays(1, &mut id));
        }
        let vao = Vao{id: id};
        vao.bind();
        vao
    }

    pub fn bind(&self) {
        verify!(gl::BindVertexArray(self.id));
    }

    pub fn unbind(&self) {
        verify!(gl::BindVertexArray(0));
    }

    pub fn draw_array(&self, mesh_mode: MeshRenderMode, faces_count: MInt) {
        let starting_index = 0;
        let vertices_count = faces_count * 3;
        let mode = mesh_mode.to_gl_type();
        verify!(gl::DrawArrays(mode, starting_index, vertices_count));
    }
}

impl Drop for Vao {
    fn drop(&mut self) {
        unsafe {
            verify!(gl::DeleteVertexArrays(1, &self.id));
        }
    }
}

pub struct Vbo {
    id: GLuint,
}

fn get_new_vbo_id() -> GLuint {
    let mut id = 0;
    unsafe {
        verify!(gl::GenBuffers(1, &mut id));
    }
    id
}

impl Vbo {
    pub fn from_data<T>(data: &[T]) -> Vbo {
        let vbo = Vbo{id: get_new_vbo_id()};
        vbo.bind();
        let size = std::mem::size_of::<T>();
        let buf_size = (data.len() * size) as GLsizeiptr;
        if data.len() != 0 {
            unsafe {
                let data_ptr = std::mem::transmute(&data[0]);
                verify!(gl::BufferData(
                    gl::ARRAY_BUFFER,
                    buf_size,
                    data_ptr,
                    gl::STATIC_DRAW,
                ));
            }
        }
        vbo
    }

    pub fn bind(&self) {
        verify!(gl::BindBuffer(gl::ARRAY_BUFFER, self.id));
    }
}

impl Drop for Vbo {
    fn drop(&mut self) {
        unsafe {
            verify!(gl::DeleteBuffers(1, &self.id));
        }
    }
}

pub fn read_pixel_bytes(
    win_size: Size2<MInt>,
    mouse_pos: ScreenPos,
) -> (MInt, MInt, MInt, MInt) {
    let height = win_size.h;
    let reverted_h = height - mouse_pos.v.y;
    let data: [u8, ..4] = [0, 0, 0, 0]; // mut
    unsafe {
        let data_ptr = std::mem::transmute(&data[0]);
        verify!(gl::ReadPixels(
            mouse_pos.v.x, reverted_h, 1, 1,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            data_ptr
        ));
    }
    (data[0] as MInt, data[1] as MInt, data[2] as MInt, data[3] as MInt)
}

pub fn get_2d_screen_matrix(win_size: Size2<MInt>) -> Matrix4<MFloat> {
    let left = 0.0;
    let right = win_size.w as MFloat;
    let bottom = 0.0;
    let top = win_size.h as MFloat;
    let near = -1.0;
    let far = 1.0;
    projection::ortho(left, right, bottom, top, near, far)
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
