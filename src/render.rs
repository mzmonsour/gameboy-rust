use std::cmp::Ordering;

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

use cgmath;
use cgmath::Matrix4;

use mem;
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
uniform vec2 translate;
uniform vec2 tex_scroll;

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

const FLIP_NONE:    u8 = 0;
const FLIP_X:       u8 = 1;
const FLIP_Y:       u8 = 2;
const FLIP_X_Y:     u8 = 3;

pub struct GbDisplay {
    vertbuf:                VertexBuffer<Vertex>,
    simple_surface_idx:     IndexBuffer<u32>,
    scroll_surface_idx:     IndexBuffer<u32>,
    sprite_8_idx:           [IndexBuffer<u32>; 4],
    sprite_16_idx:          [IndexBuffer<u32>; 4],
    color_prog:             Program,
    tex_prog:               Program,
    projection:             Matrix4<f32>,
    ly_counter:             u8,
    tex_bg:                 Texture2d,
    tex_win:                Texture2d,
    bg_last_map_addr:       u16,
    win_last_map_addr:      u16,
    last_tile_data_addr:    u16,
    sprite_cache:           Vec<SpriteData>,
}

impl GbDisplay {

    pub fn new<F>(display: &F) -> GbDisplay where F: Facade {
        let mut vertbuf = Vec::new();
        // Full texture surface
        let simple_idx = {
            // 0:   Top left
            vertbuf.push(Vertex {
                coord: [0.0, 0.0],
                tcoord: [0.0, 0.0],
            });
            // 1:   Top right
            vertbuf.push(Vertex {
                coord: [BG_SIZE as f32, 0.0],
                tcoord: [1.0, 0.0],
            });
            // 2:   Bottom right
            vertbuf.push(Vertex {
                coord: [BG_SIZE as f32, BG_SIZE as f32],
                tcoord: [1.0, 1.0],
            });
            // 3:   Bottom left
            vertbuf.push(Vertex {
                coord: [0.0, BG_SIZE as f32],
                tcoord: [0.0, 1.0],
            });
            let indices = vec![0, 1, 3, 1, 2, 3];
            IndexBuffer::immutable(display, PrimitiveType::TrianglesList, &indices).unwrap()
        };
        // Texture scrolling surface
        let scroll_idx = {
            let texw = (LCD_WIDTH as f32) / (BG_SIZE as f32);
            let texh = (LCD_HEIGHT as f32) / (BG_SIZE as f32);
            // 4:   Top left
            vertbuf.push(Vertex {
                coord: [0.0, 0.0],
                tcoord: [0.0, 0.0],
            });
            // 5:   Top right
            vertbuf.push(Vertex {
                coord: [LCD_WIDTH as f32, 0.0],
                tcoord: [texw, 0.0],
            });
            // 6:   Bottom right
            vertbuf.push(Vertex {
                coord: [LCD_WIDTH as f32, LCD_HEIGHT as f32],
                tcoord: [texw, texh],
            });
            // 7:   Bottom left
            vertbuf.push(Vertex {
                coord: [0.0, LCD_HEIGHT as f32],
                tcoord: [0.0, texh],
            });
            let indices = vec![4, 5, 7, 5, 6, 7];
            IndexBuffer::immutable(display, PrimitiveType::TrianglesList, &indices).unwrap()
        };
        // Sprite 8x8
        let sprite_8 = {
            let sprite_w = 8.0;
            let sprite_h = 8.0;
            // NO FLIP
            // 8:   Top left
            vertbuf.push(Vertex {
                coord: [0.0, 0.0],
                tcoord: [0.0, 0.0],
            });
            // 9:   Top right
            vertbuf.push(Vertex {
                coord: [sprite_w, 0.0],
                tcoord: [1.0, 0.0],
            });
            // 10:  Bottom right
            vertbuf.push(Vertex {
                coord: [sprite_w, sprite_h],
                tcoord: [1.0, 1.0],
            });
            // 11:  Bottom left
            vertbuf.push(Vertex {
                coord: [0.0, sprite_h],
                tcoord: [0.0, 1.0],
            });
            //
            // FLIP Y
            // 12:   Top left
            vertbuf.push(Vertex {
                coord: [0.0, 0.0],
                tcoord: [0.0, 1.0],
            });
            // 13:   Top right
            vertbuf.push(Vertex {
                coord: [sprite_w, 0.0],
                tcoord: [1.0, 1.0],
            });
            // 14:  Bottom right
            vertbuf.push(Vertex {
                coord: [sprite_w, sprite_h],
                tcoord: [1.0, 0.0],
            });
            // 15:  Bottom left
            vertbuf.push(Vertex {
                coord: [0.0, sprite_h],
                tcoord: [0.0, 0.0],
            });
            //
            // FLIP X
            // 16:   Top left
            vertbuf.push(Vertex {
                coord: [0.0, 0.0],
                tcoord: [1.0, 0.0],
            });
            // 17:   Top right
            vertbuf.push(Vertex {
                coord: [sprite_w, 0.0],
                tcoord: [0.0, 0.0],
            });
            // 18:  Bottom right
            vertbuf.push(Vertex {
                coord: [sprite_w, sprite_h],
                tcoord: [0.0, 1.0],
            });
            // 19:  Bottom left
            vertbuf.push(Vertex {
                coord: [0.0, sprite_h],
                tcoord: [1.0, 1.0],
            });
            //
            // FLIP BOTH
            // 20:   Top left
            vertbuf.push(Vertex {
                coord: [0.0, 0.0],
                tcoord: [1.0, 1.0],
            });
            // 21:   Top right
            vertbuf.push(Vertex {
                coord: [sprite_w, 0.0],
                tcoord: [0.0, 1.0],
            });
            // 22:  Bottom right
            vertbuf.push(Vertex {
                coord: [sprite_w, sprite_h],
                tcoord: [0.0, 0.0],
            });
            // 23:  Bottom left
            vertbuf.push(Vertex {
                coord: [0.0, sprite_h],
                tcoord: [1.0, 0.0],
            });
            let idx_noflip      = vec![8,   9,  11, 9,  10, 11];
            let idx_yflip       = vec![12,  13, 15, 13, 14, 15];
            let idx_xflip       = vec![16,  17, 19, 17, 18, 19];
            let idx_bothflip    = vec![20,  21, 23, 21, 22, 23];
            [
                IndexBuffer::immutable(display, PrimitiveType::TrianglesList, &idx_noflip).unwrap(),
                IndexBuffer::immutable(display, PrimitiveType::TrianglesList, &idx_xflip).unwrap(),
                IndexBuffer::immutable(display, PrimitiveType::TrianglesList, &idx_yflip).unwrap(),
                IndexBuffer::immutable(display, PrimitiveType::TrianglesList, &idx_bothflip).unwrap(),
            ]
        };
        // Sprite 8x16
        let sprite_16 = {
            let sprite_w = 8.0;
            let sprite_h = 16.0;
            // NO FLIP
            // 24:  Top left
            vertbuf.push(Vertex {
                coord: [0.0, 0.0],
                tcoord: [0.0, 0.0],
            });
            // 25:  Top right
            vertbuf.push(Vertex {
                coord: [sprite_w, 0.0],
                tcoord: [1.0, 0.0],
            });
            // 26:  Bottom right
            vertbuf.push(Vertex {
                coord: [sprite_w, sprite_h],
                tcoord: [1.0, 1.0],
            });
            // 27:  Bottom left
            vertbuf.push(Vertex {
                coord: [0.0, sprite_h],
                tcoord: [0.0, 1.0],
            });
            //
            // FLIP Y
            // 28:   Top left
            vertbuf.push(Vertex {
                coord: [0.0, 0.0],
                tcoord: [0.0, 1.0],
            });
            // 29:   Top right
            vertbuf.push(Vertex {
                coord: [sprite_w, 0.0],
                tcoord: [1.0, 1.0],
            });
            // 30:  Bottom right
            vertbuf.push(Vertex {
                coord: [sprite_w, sprite_h],
                tcoord: [1.0, 0.0],
            });
            // 31:  Bottom left
            vertbuf.push(Vertex {
                coord: [0.0, sprite_h],
                tcoord: [0.0, 0.0],
            });
            //
            // FLIP X
            // 32:   Top left
            vertbuf.push(Vertex {
                coord: [0.0, 0.0],
                tcoord: [1.0, 0.0],
            });
            // 33:   Top right
            vertbuf.push(Vertex {
                coord: [sprite_w, 0.0],
                tcoord: [0.0, 0.0],
            });
            // 34:  Bottom right
            vertbuf.push(Vertex {
                coord: [sprite_w, sprite_h],
                tcoord: [0.0, 1.0],
            });
            // 35:  Bottom left
            vertbuf.push(Vertex {
                coord: [0.0, sprite_h],
                tcoord: [1.0, 1.0],
            });
            //
            // FLIP BOTH
            // 36:   Top left
            vertbuf.push(Vertex {
                coord: [0.0, 0.0],
                tcoord: [1.0, 1.0],
            });
            // 37:   Top right
            vertbuf.push(Vertex {
                coord: [sprite_w, 0.0],
                tcoord: [0.0, 1.0],
            });
            // 38:  Bottom right
            vertbuf.push(Vertex {
                coord: [sprite_w, sprite_h],
                tcoord: [0.0, 0.0],
            });
            // 39:  Bottom left
            vertbuf.push(Vertex {
                coord: [0.0, sprite_h],
                tcoord: [1.0, 0.0],
            });
            let idx_noflip      = vec![24,  25, 27, 25, 26, 27];
            let idx_yflip       = vec![28,  29, 31, 29, 30, 31];
            let idx_xflip       = vec![32,  33, 35, 33, 34, 35];
            let idx_bothflip    = vec![36,  37, 39, 37, 38, 39];
            [
                IndexBuffer::immutable(display, PrimitiveType::TrianglesList, &idx_noflip).unwrap(),
                IndexBuffer::immutable(display, PrimitiveType::TrianglesList, &idx_xflip).unwrap(),
                IndexBuffer::immutable(display, PrimitiveType::TrianglesList, &idx_yflip).unwrap(),
                IndexBuffer::immutable(display, PrimitiveType::TrianglesList, &idx_bothflip).unwrap(),
            ]
        };
        // Shaders
        let colorprog = Program::from_source(display, SIMPLE_VERT, COLOR_FRAG, None).unwrap();
        let texprog = Program::from_source(display, SIMPLE_VERT, TEXTURE_FRAG, None).unwrap();
        // Projection matrices
        let projection = cgmath::ortho(0.0, LCD_WIDTH as f32, LCD_HEIGHT as f32, 0.0, 0.0, 1.0);
        let vertexbuffer = VertexBuffer::immutable(display, &vertbuf).unwrap();
        // Populate sprite cache
        let mut sprites = Vec::with_capacity(40);
        for i in 0..40 {
            sprites.push(SpriteData {
                priority: 0,
                order: 0,
                tex: Texture2d::empty(display, 8, 8).unwrap(),
                ypos: 0,
                xpos: 0,
                yflip: false,
                xflip: false,
                tile: 0,
            });
        }
        // Result
        GbDisplay {
            vertbuf: vertexbuffer,
            simple_surface_idx: simple_idx,
            scroll_surface_idx: scroll_idx,
            sprite_8_idx: sprite_8,
            sprite_16_idx: sprite_16,
            color_prog: colorprog,
            tex_prog: texprog,
            projection: projection,
            ly_counter: 0,
            tex_bg: Texture2d::empty(display, 256, 256).unwrap(),
            tex_win: Texture2d::empty(display, 256, 256).unwrap(),
            bg_last_map_addr: 0,
            win_last_map_addr: 0,
            last_tile_data_addr: 0,
            sprite_cache: sprites,
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
            projection: Into::<[[f32; 4]; 4]>::into(self.projection),
            color: color,
        };
        frame.draw(&self.vertbuf, &self.simple_surface_idx, &self.color_prog, &uniforms, &params);
    }

    pub fn draw<F>(&mut self, display: &F, frame: &mut Frame, view: Rect, mem: &mut AddressSpace) where F: Facade {
        let lcdc_reg = mem[0xFF40];
        let lcd_on                  = (lcdc_reg & 0x80) != 0;
        let win_map_addr            = if (lcdc_reg & 0x40) == 0 { 0x9800 } else { 0x9C00 };
        let win_on                  = (lcdc_reg & 0x20) != 0;
        let (tile_data, signed_idx) = if (lcdc_reg & 0x10) == 0 { (0x9000, true) }
                                        else { (0x8000, false) };
        let bg_map_addr             = if (lcdc_reg & 0x08) == 0 { 0x9800 } else { 0x9C00 };
        let sprite_height           = if (lcdc_reg & 0x04) == 0 { 1 } else { 2 };
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

        let sprite_opts = SpriteOpts {
            height_mode: sprite_height,
            palette0: sprite_palette0,
            palette1: sprite_palette1,
        };
        if sprite_on {
            build_sprites(display, mem, &sprite_opts, &mut self.sprite_cache);
        }
        self.sprite_cache.sort_by(|a, b| {
            let ord = a.xpos.cmp(&b.xpos).reverse();
            if let Ordering::Equal = ord {
                a.order.cmp(&b.order).reverse()
            } else {
                ord
            }
        });

        // Draw priority 1 sprites behind BG
        if sprite_on {
            for sprite in self.sprite_cache.iter() {
                if sprite.priority == 1 {
                    self.draw_sprite(frame, &sprite, &sprite_opts, &params);
                }
            }
        }

        // Draw BG
        if bg_on {
            let opts = TileOpts {
                map_addr: bg_map_addr,
                tile_addr: tile_data,
                signed_idx: signed_idx,
                palette: bg_palette,
            };
            let dirty_bg = {
                let observer = mem.get_observer();
                let data_dirty = if signed_idx {
                        observer.check_dirty(mem::Region::TileDataSigned)
                    } else {
                        observer.check_dirty(mem::Region::TileDataUnsigned)
                    };
                let map_dirty = if bg_map_addr == 0x9800 {
                        observer.check_dirty(mem::Region::TileMap0)
                    } else {
                        observer.check_dirty(mem::Region::TileMap1)
                    };
                map_dirty || data_dirty
            };
            if self.last_tile_data_addr != tile_data || self.bg_last_map_addr != bg_map_addr
                || dirty_bg {
                println!("Cache miss on BG: dirty_bg {}", dirty_bg);
                self.tex_bg = build_tile_tex(display, mem, &opts);
            }
            let tsx = (scroll_x as f32) / (BG_SIZE as f32);
            let tsy = (scroll_y as f32) / (BG_SIZE as f32);
            let uniforms = uniform! {
                projection: Into::<[[f32; 4]; 4]>::into(self.projection),
                tex_scroll: (tsx, tsy),
                translate: (0.0f32, 0.0f32),
                tex: Sampler::new(&self.tex_bg)
                    .magnify_filter(MagnifySamplerFilter::Nearest)
                    .minify_filter(MinifySamplerFilter::Nearest),
            };
            frame.draw(&self.vertbuf, &self.scroll_surface_idx, &self.tex_prog, &uniforms, &params);
        }

        // Draw window
        if win_on {
            println!("Drawing window...");
            let opts = TileOpts {
                map_addr: win_map_addr,
                tile_addr: tile_data,
                signed_idx: signed_idx,
                palette: bg_palette,
            };
            let dirty_win = {
                let observer = mem.get_observer();
                let data_dirty = if signed_idx {
                        observer.check_dirty(mem::Region::TileDataSigned)
                    } else {
                        observer.check_dirty(mem::Region::TileDataUnsigned)
                    };
                let map_dirty = if win_map_addr == 0x9800 {
                        observer.check_dirty(mem::Region::TileMap0)
                    } else {
                        observer.check_dirty(mem::Region::TileMap1)
                    };
                map_dirty || data_dirty
            };
            if self.last_tile_data_addr != tile_data || self.win_last_map_addr != win_map_addr
                || dirty_win {
                println!("Cache miss on Window: dirty_win {}", dirty_win);
                self.tex_win = build_tile_tex(display, mem, &opts);
            }
            let uniforms = uniform! {
                projection: Into::<[[f32; 4]; 4]>::into(self.projection),
                translate: (win_x as f32, win_y as f32),
                tex: Sampler::new(&self.tex_win)
                    .magnify_filter(MagnifySamplerFilter::Nearest)
                    .minify_filter(MinifySamplerFilter::Nearest),
            };
            frame.draw(&self.vertbuf, &self.simple_surface_idx, &self.tex_prog, &uniforms, &params);
        }

        // Draw priority 0 sprites in front of BG and Window
        if sprite_on {
            for sprite in self.sprite_cache.iter() {
                if sprite.priority == 0 {
                    self.draw_sprite(frame, &sprite, &sprite_opts, &params);
                }
            }
        }

        // Clear dirty markers on VRAM
        let observer = mem.get_observer();
        observer.clean_region(mem::Region::TileDataUnsigned);
        observer.clean_region(mem::Region::TileDataSigned);
        observer.clean_region(mem::Region::TileMap0);
        observer.clean_region(mem::Region::TileMap1);
        // Cache important VRAM pointers
        self.bg_last_map_addr = bg_map_addr;
        self.win_last_map_addr = win_map_addr;
        self.last_tile_data_addr = tile_data;
        // Restore ordering of sprite cache
        self.sprite_cache.sort_by(|a, b| {
            a.order.cmp(&b.order)
        });
    }

    fn draw_sprite(&self, frame: &mut Frame, sprite: &SpriteData, opts: &SpriteOpts, params: &DrawParameters) {
        let flip_mode = if sprite.xflip {
            if sprite.yflip {
                FLIP_X_Y
            } else {
                FLIP_X
            }
        } else if sprite.yflip {
            FLIP_Y
        } else {
            FLIP_NONE
        };
        let sprite_idx_key = if opts.height_mode > 1 {
            &self.sprite_16_idx
        } else {
            &self.sprite_8_idx
        };
        let sprite_idx = &sprite_idx_key[flip_mode as usize];
        let uniforms = uniform! {
            projection: Into::<[[f32; 4]; 4]>::into(self.projection),
            translate: (sprite.xpos as f32 - 8.0, sprite.ypos as f32 - 16.0),
            tex_scroll: (0.0f32, 0.0),
            tex: Sampler::new(&sprite.tex)
                .magnify_filter(MagnifySamplerFilter::Nearest)
                .minify_filter(MinifySamplerFilter::Nearest),
        };
        frame.draw(&self.vertbuf, sprite_idx, &self.tex_prog, &uniforms, params);
    }
}

const PALETTE_COLOR0: (f32, f32, f32, f32) = (1.0, 1.0, 1.0, 0.0);
const PALETTE_COLOR1: (f32, f32, f32, f32) = (0.4, 0.4, 0.4, 1.0);
const PALETTE_COLOR2: (f32, f32, f32, f32) = (0.1, 0.1, 0.1, 1.0);
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

struct SpriteOpts {
    height_mode: u8,
    palette0: [(f32, f32, f32, f32); 4],
    palette1: [(f32, f32, f32, f32); 4],
}

struct SpriteData {
    priority:   u8,
    order:      u8,
    tex:        Texture2d,
    ypos:       u8,
    xpos:       u8,
    yflip:      bool,
    xflip:      bool,
    tile:       u8,
}

const SPRITE_ATTR_ADDR: u16 = 0xFE00;
const SPRITE_TILE_ADDR: u16 = 0x8000;

fn build_sprites<F>(display: &F, mem: &mut AddressSpace, opts: &SpriteOpts, cache: &mut Vec<SpriteData>) where F: Facade {
    let height = opts.height_mode * 8;
    let dirty_tiles = {
        let observer = mem.get_observer();
        observer.check_dirty(mem::Region::TileDataUnsigned)
    };
    for i in 0..40 {
        let ypos = mem[SPRITE_ATTR_ADDR + i as u16 * 4];
        let xpos = mem[SPRITE_ATTR_ADDR + i as u16 * 4 + 1];
        let tile = mem[SPRITE_ATTR_ADDR + i as u16 * 4 + 2] & if opts.height_mode > 1 { 0xFE } else { 0xFF };
        let flag = mem[SPRITE_ATTR_ADDR + i as u16 * 4 + 3];
        let palette = if ((flag & 0x10) >> 4) == 0 {
            opts.palette0
        } else {
            opts.palette1
        };
        let tile_addr = SPRITE_TILE_ADDR + (tile as u16) * 16;
        // TODO: Palette changes should trigger cache misses
        if dirty_tiles || cache[i].tile != tile {
            println!("Cache miss on sprite no. {}", i);
            let mut texdata = Vec::with_capacity(height as usize);
            for _ in 0..height {
                texdata.push(Vec::with_capacity(8));
            }
            for j in 0..height {
                let lo = mem[tile_addr + (j as u16)*2];
                let hi = mem[tile_addr + (j as u16)*2 + 1];
                for k in (0..8).rev() {
                    let mask = 1 << k;
                    let color = ((lo & mask) >> k) | (((hi & mask) >> k) << 1);
                    texdata[j as usize].push(palette[color as usize]);
                }
            }
            cache[i].tex = Texture2d::with_mipmaps(display, texdata, MipmapsOption::NoMipmap).unwrap();
        }
        cache[i].priority = (flag & 0x80) >> 7;
        cache[i].order = i as u8;
        cache[i].ypos = ypos;
        cache[i].xpos = xpos;
        cache[i].yflip = (flag & 0x04) != 0;
        cache[i].xflip = (flag & 0x02) != 0;
        cache[i].tile = tile;
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
