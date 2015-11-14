use glium::VertexBuffer;
use glium::IndexBuffer;
use glium::Program;
use glium::DrawParameters;
use glium::Surface;
use glium::Frame;
use glium::Rect;
use glium::index::PrimitiveType;

pub const LCD_WIDTH: u32    = 160;
pub const LCD_HEIGHT: u32   = 144;
pub const LCD_ASPECT: f32   = (LCD_WIDTH as f32) / (LCD_HEIGHT as f32);

static SIMPLE_VERT: &'static str = r#"
#version 140

in vec2 coord;

out vec2 tex_coord;

void main() {
    tex_coord = coord;
    gl_Position = vec4(coord, 0.0, 1.0);
}
"#;

static COLOR_FRAG: &'static str = r#"
#version 140

in vec2 tex_coord;

out vec4 frag_color;

uniform vec4 color;

void main() {
    frag_color = color;
}
"#;

static TEXTURE_FRAG: &'static str = r#"
#version 140

in vec2 tex_coord;

out vec4 frag_color;

uniform sampler2D tex;

void main() {
    frag_color = texture(tex, tex_coord);
}
"#;

#[derive(Copy, Clone)]
struct Vertex {
    coord: [f32; 2],
}

implement_vertex!(Vertex, coord);

pub struct GbDisplay {
    simple_surface: VertexBuffer<Vertex>,
    simple_surface_idx: IndexBuffer<u32>,
    color_prog: Program,
    tex_prog: Program,
}

impl GbDisplay {

    pub fn new<F>(display: &F) -> GbDisplay where F: ::glium::backend::Facade {
        let (vertbuf, idxbuf) = {
            let topleft = Vertex {
                coord: [-1.0, 1.0],
            };
            let topright = Vertex {
                coord: [1.0, 1.0],
            };
            let bottomright = Vertex {
                coord: [1.0, -1.0],
            };
            let bottomleft = Vertex {
                coord: [-1.0, -1.0],
            };
            let vertices = vec![topleft, topright, bottomright, bottomleft];
            let indices = vec![0, 1, 3, 1, 2, 3];
            (
                VertexBuffer::new(display, &vertices).unwrap(),
                IndexBuffer::immutable(display, PrimitiveType::TrianglesList, &indices).unwrap()
            )
        };
        let colorprog = Program::from_source(display, SIMPLE_VERT, COLOR_FRAG, None).unwrap();
        let texprog = Program::from_source(display, SIMPLE_VERT, TEXTURE_FRAG, None).unwrap();
        GbDisplay {
            simple_surface: vertbuf,
            simple_surface_idx: idxbuf,
            color_prog: colorprog,
            tex_prog: texprog,
        }
    }

    pub fn clear_viewport(&mut self, frame: &mut Frame, view: Rect, color: (f32, f32, f32, f32)) {
        let params = DrawParameters {
            viewport: Some(view),
            .. Default::default()
        };
        let uniforms = uniform! {
            color: color,
        };
        frame.draw(&self.simple_surface, &self.simple_surface_idx, &self.color_prog, &uniforms, &params);
    }
}

pub fn calculate_viewport(width: u32, height: u32) -> Rect {
    let aspect = (width as f32) / (height as f32);
    if aspect > LCD_ASPECT {
        let fixwidth = ((height as f32) * LCD_ASPECT) as u32;
        Rect {
            left: (width - fixwidth) / 2,
            bottom: 0,
            width: fixwidth,
            height: height,
        }
    } else {
        let fixheight = ((width as f32) / LCD_ASPECT) as u32;
        Rect {
            left: 0,
            bottom: (height - fixheight) / 2,
            width: width,
            height: fixheight,
        }
    }
}
