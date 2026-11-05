use sdl3::{VideoSubsystem, render::Canvas, video::Window,     render::{Texture},
};

use crate::{RESOLUTION_X, RESOLUTION_Y, SCALE};

pub fn create_canvas(video_subsystem: &VideoSubsystem, name: &String) -> Canvas<Window> {
    const WINDOW_WIDTH: u32 = (RESOLUTION_X as u32) * SCALE;
    const WINDOW_HEIGHT: u32 = (RESOLUTION_Y as u32) * SCALE;

    let window = video_subsystem
        .window(name, WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .opengl()
        // .resizable()
        .build()
        .unwrap();

    let canvas = window.into_canvas();
    // canvas.set_logical_size(
    //     RESOLUTION_X,
    //     RESOLUTION_Y,
    //     SDL_RendererLogicalPresentation::INTEGER_SCALE,
    // );

    canvas
}

pub fn draw_screen(canvas: &mut Canvas<Window>, texture: &mut Texture<'_>, rgba: &[u8]) {
    assert_eq!(rgba.len(), (RESOLUTION_X * RESOLUTION_Y * 4) as usize);
    texture
        .update(None, rgba, (RESOLUTION_X * 4) as usize)
        .unwrap();
    canvas.clear();
    canvas.copy(texture, None, None).unwrap();
}

pub fn to_rgba(shades: &[u8], out: &mut [u8], drawn: &[bool]) {
    // const PALETTE: [[u8; 4]; 4] = [
    //     [0xC8, 0xCC, 0xC8, 0xFF],
    //     [0xA0, 0xA4, 0xA0, 0xFF],
    //     [0x68, 0x6C, 0x68, 0xFF],
    //     [0x30, 0x34, 0x30, 0xFF],
    // ];
    const PALETTE: [[u8; 4]; 4] = [
        [0xFF, 0xF6, 0xD3, 0xFF],
        [0xF9, 0xA8, 0x75, 0xFF],
        [0xEB, 0xCB, 0x6F, 0xFF],
        [0x7C, 0x3F, 0x58, 0xFF],
    ];

    for (i, (&shade, &is_drawn)) in shades.iter().zip(drawn).enumerate() {
        let rgba = if is_drawn {
            &PALETTE[(shade & 3) as usize]
        } else {
            &[0x00, 0x00, 0x00, 0xFF]
        };

        out[i * 4..i * 4 + 4].copy_from_slice(rgba);
    }
}
