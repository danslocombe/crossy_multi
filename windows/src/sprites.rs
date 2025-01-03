use core::str;
use std::{collections::BTreeMap, mem::MaybeUninit};

use crossy_multi_core::math::V2;
use raylib_sys::Color;


static mut SPRITE_FRAMES: MaybeUninit<BTreeMap<String, Vec<raylib_sys::Texture2D>>> = MaybeUninit::uninit();

pub fn init_sprites() {
    unsafe {
        // @Perf should be enum instead of hashmap
        let map = BTreeMap::new();
        SPRITE_FRAMES = MaybeUninit::new(map);
        load_frames("../web-client/static/sprites/spr_frog.png", None);
        load_frames("../web-client/static/sprites/spr_frog_dead.png", None);
        load_frames("../web-client/static/sprites/spr_frog_dialogue.png", Some(2));

        load_frames("../web-client/static/sprites/spr_mouse.png", None);
        load_frames("../web-client/static/sprites/spr_mouse_dead.png", None);
        load_frames("../web-client/static/sprites/spr_mouse_dialogue_cute.png", Some(2));

        load_frames("../web-client/static/sprites/spr_bird.png", None);
        load_frames("../web-client/static/sprites/spr_bird_dead.png", None);
        load_frames("../web-client/static/sprites/spr_bird_dialogue_cute.png", Some(2));

        load_frames("../web-client/static/sprites/spr_snake.png", None);
        load_frames("../web-client/static/sprites/spr_snake_dead.png", None);
        load_frames("../web-client/static/sprites/spr_snake_dialogue.png", Some(2));

        load_frames("../web-client/static/sprites/spr_duck.png", None);
        load_frames("../web-client/static/sprites/spr_duck_dead.png", None);
        load_frames("../web-client/static/sprites/spr_duck_dialogue.png", Some(2));

        load_frames("../web-client/static/sprites/spr_mouse.png", None);
        load_frames("../web-client/static/sprites/spr_mouse_dead.png", None);
        load_frames("../web-client/static/sprites/spr_mouse_dialogue_cute.png", Some(2));

        load_frames("../web-client/static/sprites/spr_woshette.png", None);
        load_frames("../web-client/static/sprites/spr_woshette_dead.png", None);
        load_frames("../web-client/static/sprites/spr_woshette_dialogue.png", Some(2));

        load_frames("../web-client/static/sprites/spr_frog_alt.png", None);
        load_frames("../web-client/static/sprites/spr_frog_alt_dead.png", None);
        load_frames("../web-client/static/sprites/spr_frog_alt_dialogue.png", Some(2));

        load_frames("../web-client/static/sprites/spr_frog_3.png", None);
        load_frames("../web-client/static/sprites/spr_frog_3_dead.png", None);
        load_frames("../web-client/static/sprites/spr_frog_3_dialogue.png", Some(2));

        load_frames("../web-client/static/sprites/spr_sausage.png", None);
        load_frames("../web-client/static/sprites/spr_sausage_dead.png", None);
        load_frames("../web-client/static/sprites/spr_sausage_dialogue.png", Some(2));

        load_frames("../web-client/static/sprites/spr_shadow.png", None);
        load_frames("../web-client/static/sprites/spr_crown.png", None);

        load_frames("../web-client/static/sprites/spr_block.png", None);
        load_frames("../web-client/static/sprites/spr_barrier.png", None);

        load_frames("../web-client/static/sprites/spr_foliage.png", Some(6));
        load_frames("../web-client/static/sprites/spr_stand.png", Some(1));
        load_frames("../web-client/static/sprites/spr_tree_top.png", Some(6));

        load_frames("../web-client/static/sprites/spr_car_flipped.png", Some(4));
        load_frames("../web-client/static/sprites/spr_log.png", None);

        load_frames("../web-client/static/sprites/spr_dust.png", Some(4));
        load_frames("../web-client/static/sprites/spr_bubble.png", Some(5));

        load_frames("../web-client/static/sprites/spr_countdown.png", Some(4));
        load_frames("../web-client/static/sprites/spr_winner.png", Some(1));
        load_frames("../web-client/static/sprites/spr_no_winner.png", Some(1));
        load_frames("../web-client/static/sprites/spr_champion.png", Some(1));
        load_frames("../web-client/static/sprites/spr_roadtoads.png", Some(1));

        load_frames("../web-client/static/sprites/spr_wizard_hat.png", Some(1));

        load_frames("../web-client/static/sprites/spr_raft.png", Some(1));
        load_frames("../web-client/static/sprites/spr_raft_sail_frame.png", Some(1));

        load_frames("../web-client/static/sprites/spr_font_linsenn_m5x7_numbers.png", Some(10));
    }
}

fn load_frames(filename: &str, p_frame_count: Option<usize>) {
    let frames = unsafe { load_frames_unsafe(filename, p_frame_count) };

    let path = std::path::Path::new(filename);
    let mut name = path.file_stem().unwrap().to_str().unwrap();
    if name.starts_with("spr_") {
        name = &name["spr_".len()..];
    }
    unsafe {
        SPRITE_FRAMES.assume_init_mut().insert(name.to_owned(), frames);
    }
}

unsafe fn load_frames_unsafe(filename: &str, p_frame_count: Option<usize>) -> Vec<raylib_sys::Texture2D> {
    let image = raylib_sys::LoadImage(crate::c_str_temp(filename));

    let mut frame_count: usize = 0;
    if (p_frame_count == None) {
        frame_count = (image.width / image.height) as usize;
    }
    else {
        frame_count = p_frame_count.unwrap();
    }

    let frame_w = image.width / (frame_count as i32);

    let mut frames = Vec::new();
    for iu in 0..frame_count {
        let i = iu as i32;
        let xoff: f32 = (i * frame_w) as f32;
        let frame_image = raylib_sys::ImageFromImage(
            image,
            raylib_sys::Rectangle {
                x: xoff,
                y: 0.0,
                width: frame_w as f32,
                height: image.height as f32 });

        frames.push(raylib_sys::LoadTextureFromImage(frame_image));

        raylib_sys::UnloadImage(frame_image);
    }

    raylib_sys::UnloadImage(image);
    frames
}

pub fn get_sprite(name: &str) -> &[raylib_sys::Texture2D] {
    unsafe { 
        let frames = SPRITE_FRAMES.assume_init_ref();
        let frame_vec = frames.get(name).unwrap_or_else(|| {
            println!("Could not find {}", name);
            frames.get("unknown").expect(&format!("Could not find {}", name))
        });
        &frame_vec[..]
    }
}

pub fn draw(name: &str, image_index: usize, x: f32, y: f32) {
    let spr = &get_sprite(name)[image_index];
    unsafe {
        raylib_sys::DrawTexture(
            *spr,
            x as i32,
            y as i32,
            crate::WHITE);
    }
}

pub fn draw_ex(name: &str, image_index: usize, pos: V2, rotation: f32, scale: f32) {
    let sprite = &get_sprite(name)[image_index];
    unsafe {
        let rect = raylib_sys::Rectangle{
            x: 0.0,
            y: 0.0,
            width: sprite.width as f32,
            height: sprite.height as f32,
        };

        let dest = raylib_sys::Rectangle{
            x: pos.x,
            y: pos.y,
            width: sprite.width as f32 * scale,
            height: sprite.height as f32 * scale,
        };

        raylib_sys::DrawTexturePro(
            *sprite,
            rect,
            dest,
            raylib_sys::Vector2::new(sprite.width as f32 * 0.5, sprite.height as f32 * 0.5),
            rotation,
            crate::WHITE);
    }
}

pub fn draw_with_flip(name: &str, image_index: usize, x: f32, y: f32, x_flip: bool) {
    let sprite = get_sprite(name)[image_index];
    let x_flip_f = if x_flip {-1.0} else {1.0};
    let rect = raylib_sys::Rectangle{
        x: 0.0,
        y: 0.0,
        width: sprite.width as f32 * x_flip_f,
        height: sprite.height as f32,
    };

    let dest = raylib_sys::Rectangle{
        x,
        y,
        width: sprite.width as f32,
        height: sprite.height as f32,
    };

    unsafe {
        raylib_sys::DrawTexturePro(
            sprite,
            rect,
            dest,
            raylib_sys::Vector2::zero(),
            0.0,
            crate::WHITE);
    }
}

pub fn draw_scaled(name: &str, image_index: usize, x: f32, y: f32, scale: f32) {
    draw_scaled_tinted(name, image_index, x, y, scale, crate::WHITE);
}

pub fn draw_scaled_tinted(name: &str, image_index: usize, x: f32, y: f32, scale: f32, tint: Color) {
    let sprite = get_sprite(name)[image_index];
    let rect = raylib_sys::Rectangle{
        x: 0.0,
        y: 0.0,
        width: sprite.width as f32,
        height: sprite.height as f32,
    };

    let dest = raylib_sys::Rectangle{
        x,
        y,
        width: sprite.width as f32 * scale,
        height: sprite.height as f32 * scale,
    };

    unsafe {
        raylib_sys::DrawTexturePro(
            sprite,
            rect,
            dest,
            raylib_sys::Vector2::zero(),
            0.0,
            tint);
    }
}