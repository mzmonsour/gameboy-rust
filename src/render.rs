use glium::VertexBuffer;
use glium::IndexBuffer;
use glium::Program;
use glium::DrawParameters;
use glium::Surface;
use glium::Frame;
use glium::Rect;
use glium::Texture2d;
use glium::Blend;
use glium::index::PrimitiveType;
use glium::backend::Facade;
use glium::texture::MipmapsOption;
use glium::uniforms::{Sampler, MagnifySamplerFilter, MinifySamplerFilter};

use nalgebra::Mat4;
use nalgebra::OrthoMat3;

use mem::MemSection;
use mem::AddressSpace;

pub const LCD_WIDTH: u32    = 160;
pub const LCD_HEIGHT: u32   = 144;
pub const LCD_ASPECT: f32   = (LCD_WIDTH as f32) / (LCD_HEIGHT as f32);

pub const BG_SIZE: u32      = 256;

/// Period of the V-Blank in ns. V-Blank frequency is ~59.7 Hz
pub const VBLANK_PERIOD: u64 = 16_750_419;

/// Period of the H-Blank in ns. H-Blank frequency is ~9198 Hz
pub const HBLANK_PERIOD: u64 = 108_719;

static SIMPLE_VERT: &'static str = r#"
#version 140

in vec2 coord;
in vec2 tcoord;

out vec2 tex_coord;

uniform mat4 projection;
uniform vec2 translate = vec2(0.0,  0.0);
uniform vec2 tex_scroll = vec2(0.0, 0.0);

void main() {
    tex_coord = tcoord + tex_scroll;
    gl_Position = projection * vec4(coord + translate, 0.0, 1.0);
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
    tcoord: [f32; 2],
}

implement_vertex!(Vertex, coord, tcoord);

pub struct GbDisplay {
    simple_surface: VertexBuffer<Vertex>,
    simple_surface_idx: IndexBuffer<u32>,
    scroll_surface: VertexBuffer<Vertex>,
    scroll_surface_idx: IndexBuffer<u32>,
    color_prog: Program,
    tex_prog: Program,
    projection: Mat4<f32>,
    ly_counter: u8,
}

impl GbDisplay {

    pub fn new<F>(display: &F) -> GbDisplay where F: Facade {
        // Full texture surface
        let (vertbuf, idxbuf) = {
            let topleft = Vertex {
                coord: [0.0, 0.0],
                tcoord: [0.0, 0.0],
            };
            let topright = Vertex {
                coord: [BG_SIZE as f32, 0.0],
                tcoord: [1.0, 0.0],
            };
            let bottomright = Vertex {
                coord: [BG_SIZE as f32, BG_SIZE as f32],
                tcoord: [1.0, 1.0],
            };
            let bottomleft = Vertex {
                coord: [0.0, BG_SIZE as f32],
                tcoord: [0.0, 1.0],
            };
            let vertices = vec![topleft, topright, bottomright, bottomleft];
            let indices = vec![0, 1, 3, 1, 2, 3];
            (
                VertexBuffer::new(display, &vertices).unwrap(),
                IndexBuffer::immutable(display, PrimitiveType::TrianglesList, &indices).unwrap()
            )
        };
        // Texture scrolling surface
        let (scroll_buff, scroll_idx) = {
            let texw = (LCD_WIDTH as f32) / (BG_SIZE as f32);
            let texh = (LCD_HEIGHT as f32) / (BG_SIZE as f32);
            let topleft = Vertex {
                coord: [0.0, 0.0],
                tcoord: [0.0, 0.0],
            };
            let topright = Vertex {
                coord: [LCD_WIDTH as f32, 0.0],
                tcoord: [texw, 0.0],
            };
            let bottomright = Vertex {
                coord: [LCD_WIDTH as f32, LCD_HEIGHT as f32],
                tcoord: [texw, texh],
            };
            let bottomleft = Vertex {
                coord: [0.0, LCD_HEIGHT as f32],
                tcoord: [0.0, texh],
            };
            let vertices = vec![topleft, topright, bottomright, bottomleft];
            let indices = vec![0, 1, 3, 1, 2, 3];
            (
                VertexBuffer::new(display, &vertices).unwrap(),
                IndexBuffer::immutable(display, PrimitiveType::TrianglesList, &indices).unwrap()
            )
        };
        // Shaders
        let colorprog = Program::from_source(display, SIMPLE_VERT, COLOR_FRAG, None).unwrap();
        let texprog = Program::from_source(display, SIMPLE_VERT, TEXTURE_FRAG, None).unwrap();
        // Projection matrices
        let projection = {
            let orthomat = OrthoMat3::new(LCD_WIDTH as f32, LCD_HEIGHT as f32, 0.0, 1.0);
            // Reverse y coord, and translate origin to top left
            let adjust = Mat4::new(
                1.0f32, 0.0, 0.0, (LCD_WIDTH as f32) / -2.0,
                0.0, -1.0, 0.0, (LCD_HEIGHT as f32) / 2.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0
                );
            orthomat.to_mat() * adjust
        };
        // Result
        GbDisplay {
            simple_surface: vertbuf,
            simple_surface_idx: idxbuf,
            scroll_surface: scroll_buff,
            scroll_surface_idx: scroll_idx,
            color_prog: colorprog,
            tex_prog: texprog,
            projection: projection,
            ly_counter: 0,
        }
    }

    pub fn inc_ly_counter(&mut self) -> u8 {
        if self.ly_counter >= 153 {
            self.ly_counter = 0;
        } else {
            self.ly_counter += 1;
        }
        self.ly_counter
    }

    pub fn set_ly_vblank(&mut self) -> u8 {
        self.ly_counter = 144;
        self.ly_counter
    }

    pub fn clear_viewport(&mut self, frame: &mut Frame, view: Rect, color: (f32, f32, f32, f32)) {
        let params = DrawParameters {
            viewport: Some(view),
            .. Default::default()
        };
        let uniforms = uniform! {
            projection: self.projection,
            color: color,
        };
        frame.draw(&self.simple_surface, &self.simple_surface_idx, &self.color_prog, &uniforms, &params);
    }

    pub fn draw<F>(&mut self, display: &F, frame: &mut Frame, view: Rect, mem: &AddressSpace) where F: Facade {
        let lcdc_reg = mem[0xFF40];
        let lcd_on                  = (lcdc_reg & 0x80) != 0;
        let win_map_addr            = if (lcdc_reg & 0x40) == 0 { 0x9800 } else { 0x9C00 };
        let win_on                  = (lcdc_reg & 0x20) != 0;
        let (tile_data, signed_idx) = if (lcdc_reg & 0x10) == 0 { (0x9000, true) }
                                        else { (0x8000, false) };
        let bg_map_addr             = if (lcdc_reg & 0x08) == 0 { 0x9800 } else { 0x9C00 };
        let sprite_height           = if (lcdc_reg & 0x04) == 0 { 8 } else { 16 };
        let sprite_on               = (lcdc_reg & 0x02) != 0;
        let bg_on                   = (lcdc_reg & 0x01) != 0;
        let scroll_y = mem[0xFF42];
        let scroll_x = mem[0xFF43];
        let bg_palette = build_palette(mem[0xFF47]);
        let sprite_palette0 = build_palette(mem[0xFF48]);
        let sprite_palette1 = build_palette(mem[0xFF49]);
        let win_y = mem[0xFF4A];
        let win_x = mem[0xFF4B];

        let params = DrawParameters {
            blend: Blend::alpha_blending(),
            viewport: Some(view),
            .. Default::default()
        };

        // Draw BG
        if bg_on {
            let opts = TileOpts {
                map_addr: bg_map_addr,
                tile_addr: tile_data,
                signed_idx: signed_idx,
                palette: bg_palette,
            };
            let bg_tex = build_tile_tex(display, mem, &opts);
            let tsx = (scroll_x as f32) / (BG_SIZE as f32);
            let tsy = (scroll_y as f32) / (BG_SIZE as f32);
            let uniforms = uniform! {
                projection: self.projection,
                tex_scroll: (tsx, tsy),
                tex: Sampler::new(&bg_tex)
                    .magnify_filter(MagnifySamplerFilter::Nearest)
                    .minify_filter(MinifySamplerFilter::Nearest),
            };
            frame.draw(&self.scroll_surface, &self.scroll_surface_idx, &self.tex_prog, &uniforms, &params);
        }

        // Draw window
        if win_on {
            let opts = TileOpts {
                map_addr: win_map_addr,
                tile_addr: tile_data,
                signed_idx: signed_idx,
                palette: bg_palette,
            };
            let win_tex = build_tile_tex(display, mem, &opts);
            let uniforms = uniform! {
                projection: self.projection,
                translate: (win_x as f32, win_y as f32),
                tex: Sampler::new(&win_tex)
                    .magnify_filter(MagnifySamplerFilter::Nearest)
                    .minify_filter(MinifySamplerFilter::Nearest),
            };
            frame.draw(&self.simple_surface, &self.simple_surface_idx, &self.tex_prog, &uniforms, &params);
        }
    }
}

const PALETTE_COLOR0: (f32, f32, f32, f32) = (1.0, 1.0, 1.0, 0.0);
const PALETTE_COLOR1: (f32, f32, f32, f32) = (0.25, 0.25, 0.25, 1.0);
const PALETTE_COLOR2: (f32, f32, f32, f32) = (0.75, 0.75, 0.75, 1.0);
const PALETTE_COLOR3: (f32, f32, f32, f32) = (0.0, 0.0, 0.0, 1.0);
const PALETTE_COLORTEST: (f32, f32, f32, f32) = (1.0, 0.5, 0.75, 1.0);

fn build_palette(bits: u8) -> [(f32, f32, f32, f32); 4] {
    let c0 = (bits & 0x03);
    let c1 = (bits & 0x0C) >> 2;
    let c2 = (bits & 0x30) >> 4;
    let c3 = (bits & 0xC0) >> 6;
    let mut colors = [PALETTE_COLOR0; 4];
    colors[0] = match c0 {
        0 => PALETTE_COLOR0,
        1 => PALETTE_COLOR1,
        2 => PALETTE_COLOR2,
        3 => PALETTE_COLOR3,
        _ => PALETTE_COLORTEST,
    };
    colors[1] = match c1 {
        0 => PALETTE_COLOR0,
        1 => PALETTE_COLOR1,
        2 => PALETTE_COLOR2,
        3 => PALETTE_COLOR3,
        _ => PALETTE_COLORTEST,
    };
    colors[2] = match c2 {
        0 => PALETTE_COLOR0,
        1 => PALETTE_COLOR1,
        2 => PALETTE_COLOR2,
        3 => PALETTE_COLOR3,
        _ => PALETTE_COLORTEST,
    };
    colors[3] = match c3 {
        0 => PALETTE_COLOR0,
        1 => PALETTE_COLOR1,
        2 => PALETTE_COLOR2,
        3 => PALETTE_COLOR3,
        _ => PALETTE_COLORTEST,
    };
    colors
}

struct TileOpts {
    map_addr: u16,
    tile_addr: u16,
    signed_idx: bool,
    palette: [(f32, f32, f32, f32); 4],
}

fn build_tile_tex<F>(display: &F, mem: &AddressSpace, opts: &TileOpts) -> Texture2d where F: Facade {
    // Data layout: data[y][x], origin top left
    let mut data: Vec<Vec<(f32, f32, f32, f32)>> = Vec::with_capacity(256);
    for i in 0..256 {
        data.push(Vec::with_capacity(256));
    }
    for i in 0..1024 {
        let offset: i32 = if opts.signed_idx {
            // Abuse conversion operations
            let tmp = mem[opts.map_addr + i] as i8;
            tmp as i32
        } else {
            mem[opts.map_addr + i] as i32
        };
        let addr = (opts.tile_addr as i32 + offset*16) as u16;
        let y_coord = (i / 32) as usize;
        // Pixel format: big endian pixel pos
        for j in 0..8 {
            let lo = mem[addr + j*2];
            let hi = mem[addr + j*2 + 1];
            for k in (0..8).rev() {
                let mask = 1 << k;
                let color = ((lo & mask) >> k) | (((hi & mask) >> k) << 1);
                data[y_coord*8 + j as usize].push(opts.palette[color as usize]);
            }
        }
    }
    Texture2d::with_mipmaps(display, data, MipmapsOption::NoMipmap).unwrap()
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
