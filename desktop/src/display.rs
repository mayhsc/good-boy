use egui_sdl3::sdl3::{
    VideoSubsystem,
    pixels::PixelFormat,
    render::Texture,
    render::{Canvas, ScaleMode},
    video::Window,
};

use crate::{RESOLUTION_X, RESOLUTION_Y, SCALE, palette::ColorPalette, session::Session};

pub struct Screen {
    canvas: Canvas<Window>,
    rgba: Vec<u8>,
}

impl Screen {
    pub fn new(video: &VideoSubsystem, title: &str) -> Self {
        Self {
            canvas: create_canvas(video, &title.to_owned()),
            rgba: vec![0; (RESOLUTION_X * RESOLUTION_Y * 4) as usize],
        }
    }

    pub fn draw(&mut self, session: &Session, palette: &ColorPalette) {
        to_rgba(
            session.frame_buffer(),
            &mut self.rgba,
            session.drawn(),
            palette,
        );
        let creator = self.canvas.texture_creator();
        let mut texture = creator
            .create_texture_streaming(PixelFormat::RGBA32, RESOLUTION_X, RESOLUTION_Y)
            .unwrap();
        texture.set_scale_mode(ScaleMode::Nearest);
        draw_screen(&mut self.canvas, &mut texture, &self.rgba);
        self.canvas.present();
    }

    pub fn clear(&mut self) {
        self.canvas.clear();
        self.canvas.present();
    }

    pub fn set_title(&mut self, title: &str) {
        self.canvas.window_mut().set_title(title).unwrap();
    }

    pub fn confirm_quit_without_saving(&self) -> bool {
        crate::save::show_save_error_dialog(&self.canvas)
    }
}

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

pub fn draw_screen(canvas: &mut Canvas<Window>, texture: &mut Texture, rgba: &[u8]) {
    assert_eq!(rgba.len(), (RESOLUTION_X * RESOLUTION_Y * 4) as usize);
    texture
        .update(None, rgba, (RESOLUTION_X * 4) as usize)
        .unwrap();
    canvas.clear();
    canvas.copy(texture, None, None).unwrap();
}

pub fn to_rgba(shades: &[u8], out: &mut [u8], drawn: &[bool], palette: &ColorPalette) {
    for (i, (&shade, &is_drawn)) in shades.iter().zip(drawn).enumerate() {
        let rgba = if is_drawn {
            &palette.colors[(shade as usize) % palette.colors.len()]
        } else {
            &[0, 0, 0, 255]
        };

        out[i * 4..i * 4 + 4].copy_from_slice(rgba);
    }
}
