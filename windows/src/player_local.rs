use crossy_multi_core::{crossy_ruleset::{player_in_lobby_ready_zone, AliveState}, game, map::RowType, math::V2, player::PlayerStatePublic, timeline::{Timeline, TICK_INTERVAL_US}, CoordPos, GameState, Input, PlayerId, PlayerInputs, Pos};
use froggy_rand::FroggyRand;

use crate::{client::VisualEffects, console, diff, entities::{Bubble, Corpse, Dust, Entity, EntityContainer, EntityType, IsEntity}, key_pressed, lerp_snap, sprites};

#[derive(Debug)]
pub struct PlayerLocal {
    pub entity_id: i32,
    pub player_id: PlayerId,
    pub pos: V2,
    pub moving: bool,
    pub x_flip: bool,
    pub image_index: i32,
    pub buffered_input: Input,
    pub created_corpse: bool,
    pub t : i32,
    pub skin: Skin,
}

const MOVE_T : i32 = 7 * (1000 * 1000 / 60);
const PLAYER_FRAME_COUNT: i32 = 5;

#[derive(Default)]
pub struct PlayerInputController {
    arrow_key_player: Option<PlayerId>,
    wasd_player: Option<PlayerId>,
    controller_a_player: Option<PlayerId>,
    controller_b_player: Option<PlayerId>,
    controller_c_player: Option<PlayerId>,
    controller_d_player: Option<PlayerId>,
}

impl PlayerInputController {
    pub fn tick(&mut self, timeline: &mut Timeline, players_local: &mut EntityContainer<PlayerLocal>) -> PlayerInputs {
        let mut player_inputs = PlayerInputs::default();

        {
            // Arrows
            let mut input = Input::None;
            if (key_pressed(raylib_sys::KeyboardKey::KEY_LEFT)) {
                input = game::Input::Left;
            }
            if (key_pressed(raylib_sys::KeyboardKey::KEY_RIGHT)) {
                input = game::Input::Right;
            }
            if (key_pressed(raylib_sys::KeyboardKey::KEY_UP)) {
                input = game::Input::Up;
            }
            if (key_pressed(raylib_sys::KeyboardKey::KEY_DOWN)) {
                input = game::Input::Down;
            }

            Self::process_input(&mut self.arrow_key_player, input, &mut player_inputs, timeline, players_local);
        }

        {
            // WASD
            let mut input = Input::None;
            if (key_pressed(raylib_sys::KeyboardKey::KEY_A)) {
                input = game::Input::Left;
            }
            if (key_pressed(raylib_sys::KeyboardKey::KEY_D)) {
                input = game::Input::Right;
            }
            if (key_pressed(raylib_sys::KeyboardKey::KEY_W)) {
                input = game::Input::Up;
            }
            if (key_pressed(raylib_sys::KeyboardKey::KEY_S)) {
                input = game::Input::Down;
            }

            Self::process_input(&mut self.wasd_player, input, &mut player_inputs, timeline, players_local);
        }

        player_inputs
    }

    pub fn process_input(id_registration: &mut Option<PlayerId>, input: Input, player_inputs: &mut PlayerInputs, timeline: &mut Timeline, players_local: &mut EntityContainer<PlayerLocal>) {
        if let Some(pid) = *id_registration {
            let player = players_local.inner.iter_mut().find(|x| x.player_id == pid).unwrap();
            player.update_inputs(&*timeline, player_inputs, input);
        }
        else if input != Input::None{
            // Create player.
            let top = timeline.top_state();
            if let Some(new_id) = top.player_states.next_free() {
                *id_registration = Some(new_id);
                drop(top);
                timeline.add_player(new_id, Pos::new_coord(7, 7));

                let top = timeline.top_state();
                let player_state = top.player_states.get(new_id).unwrap().to_public(top.get_round_id(), top.time_us, &timeline.map);
                let player_local = players_local.create(Pos::Absolute(V2::default()));
                player_local.set_from(&player_state);
                let rand = FroggyRand::new(timeline.len() as u64);
                player_local.skin = Skin::from_enum(*rand.choose((), &crate::player_local::g_all_skins));
            }
            else {
                console::info("Unable to create another player");
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Skin {
    pub player_skin: PlayerSkin,
    pub sprite: &'static str,
    pub dead_sprite: &'static str,
    pub dialogue_sprite: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub enum PlayerSkin {
    Frog,
    Bird,
    Snake,
    Duck,
    Mouse,
    Wosh,
    FrogAlt,
    Frog3,
}

pub const g_all_skins: [PlayerSkin; 8] = [
    PlayerSkin::Frog,
    PlayerSkin::Bird,
    PlayerSkin::Snake,
    PlayerSkin::Duck,
    PlayerSkin::Mouse,
    PlayerSkin::Wosh,
    PlayerSkin::FrogAlt,
    PlayerSkin::Frog3,
];

impl Default for Skin {
    fn default() -> Self {
        Self {
            player_skin: PlayerSkin::Frog,
            sprite: "frog",
            dead_sprite: "frog_dead",
            dialogue_sprite: "frog_dialogue",
        }
    }
}

impl Skin {
    pub fn from_enum(player_skin: PlayerSkin) -> Self {
        match player_skin {
            PlayerSkin::Frog => Self {
                player_skin,
                sprite: "frog",
                dead_sprite: "frog_dead",
                dialogue_sprite: "frog_dialogue",
            },
            PlayerSkin::Bird => Self {
                player_skin,
                sprite: "bird",
                dead_sprite: "bird_dead",
                dialogue_sprite: "bird_dialogue_cute",
            },
            PlayerSkin::Snake => Self {
                player_skin,
                sprite: "snake",
                dead_sprite: "snake_dead",
                dialogue_sprite: "snake_dialogue",
            },
            PlayerSkin::Duck => Self {
                player_skin,
                sprite: "duck",
                dead_sprite: "duck_dead",
                dialogue_sprite: "duck_dialogue",
            },
            PlayerSkin::Mouse => Self {
                player_skin,
                sprite: "mouse",
                dead_sprite: "mouse_dead",
                dialogue_sprite: "mouse_dialogue_cute",
            },
            PlayerSkin::Wosh => Self {
                player_skin,
                sprite: "woshette",
                dead_sprite: "frog_dead",
                dialogue_sprite: "frog_dialogue",
            },
            PlayerSkin::FrogAlt => Self {
                player_skin,
                sprite: "frog_alt",
                dead_sprite: "frog_dead",
                dialogue_sprite: "frog_dialogue",
            },
            PlayerSkin::Frog3 => Self {
                player_skin,
                sprite: "frog_3",
                dead_sprite: "frog_dead",
                dialogue_sprite: "frog_dialogue",
            },
        }
    }
}

impl PlayerLocal {
    pub fn new(entity_id: i32, pos: V2,) -> Self {
        Self {
            entity_id,
            player_id: PlayerId(0),
            pos,
            moving: false,
            x_flip: false,
            image_index: 0,
            buffered_input: Input::None,
            created_corpse: false,
            t: 0,
            skin: Skin::default(),
        }
    }

    pub fn reset(&mut self) {
        self.created_corpse = false;
    }

    pub fn set_from(&mut self, state: &PlayerStatePublic) {
        self.player_id = PlayerId(state.id);
        self.pos = V2::new(state.x as f32, state.y as f32);
    }

    pub fn update_inputs(&mut self, timeline: &Timeline, player_inputs: &mut PlayerInputs, input: Input) {
        if (input != Input::None) {
            self.buffered_input = input;

        }

        if (input == Input::Left) {
            self.x_flip = true;
        }

        if (input == Input::Right) {
            self.x_flip = false;
        }

        let top = timeline.top_state();
        if (top.player_states.get(self.player_id).unwrap().can_move()) {
            player_inputs.set(self.player_id, self.buffered_input);
            self.buffered_input = Input::None;
        }
    }

    pub fn tick(
        &mut self,
        player_state: &PlayerStatePublic,
        alive_state: AliveState,
        timeline: &Timeline,
        visual_effects: &mut VisualEffects,
        dust: &mut EntityContainer<Dust>,
        bubbles: &mut EntityContainer<Bubble>,
        corpses: &mut EntityContainer<Corpse>) {
        self.t += 1;

        let x0 = player_state.x as f32;
        let y0 = player_state.y as f32;

        let mut x: f32 = 0.0;
        let mut y: f32 = 0.0;
        if (player_state.moving) {
            let lerp_t = 1.0 - (player_state.remaining_move_dur as f32 / MOVE_T as f32);

            let x1 = player_state.t_x as f32;
            let y1 = player_state.t_y as f32;

            self.image_index = (self.image_index + 1);
            if (self.image_index >= PLAYER_FRAME_COUNT) {
                self.image_index = PLAYER_FRAME_COUNT - 1;
            }

            x = x0 + lerp_t * (x1 - x0);
            y = y0 + lerp_t * (y1 - y0);
        }
        else {
            let new_p = lerp_snap(self.pos.x, self.pos.y, x0, y0);
            x = new_p.x;
            y = new_p.y;

            let delta = 8.0 * 0.01;
            if (diff(x, self.pos.x) > delta || diff(y, self.pos.y) > delta) {
                self.image_index = (self.image_index + 1) % PLAYER_FRAME_COUNT;
            }
            else {
                self.image_index = 0;
            }
        }

        if (player_state.moving && !self.moving) {
            // Started moving, do effects.
            let rand = FroggyRand::from_hash((self.player_id.0, self.t));
            for i in 0..2 {
                let rand = rand.subrand(i);
                let dust_off = rand.gen_unit("off") * 3.0;
                let dust_dir = rand.gen_unit("dir") * 3.141 * 2.0;
                let pos = self.pos * 8.0 + V2::new(4.0, 4.0) + V2::norm_from_angle(dust_dir as f32) * dust_off as f32;
                //let pos = self.pos * 8.0 + V2::norm_from_angle(dust_dir as f32) * dust_off as f32;
                let eid = dust.create_entity(Entity {
                    id: 0,
                    entity_type: EntityType::Dust,
                    pos: Pos::Absolute(pos),
                });
                let dust_part = dust.get_mut(eid).unwrap();
                dust_part.image_index = rand.gen_usize_range("frame", 0, 3) as i32;
                dust_part.scale = (0.5 + rand.gen_unit("scale") * 0.6) as f32;
            }
        }

        if (alive_state == AliveState::Dead && !self.created_corpse) {
            self.created_corpse = true;

            //let target_pos = V2::new((player_state.t_x * 8.0) as f32, player_state.t_y as f32 * 8.0);
            let corpse_pos = if player_state.moving {
                V2::new(player_state.t_x as f32, player_state.t_y as f32)
            }
            else {
                V2::new(player_state.x as f32, player_state.y as f32)
            } * 8.0;

            let top_state = timeline.top_state();
            let row = timeline.map.get_row(top_state.rules_state.fst.get_round_id(), player_state.y);
            if let RowType::River(_) = row.row_type {
                // Drowning.
                let rand = FroggyRand::from_hash((self.player_id.0, self.t));
                for i in 0..2 {
                    let rand = rand.subrand(i);
                    let dust_off = rand.gen_unit("off") * 3.0;
                    let dust_dir = rand.gen_unit("dir") * 3.141 * 2.0;
                    let pos = corpse_pos * 8.0 + V2::new(4.0, 4.0) + V2::norm_from_angle(dust_dir as f32) * dust_off as f32;
                    //let pos = self.pos * 8.0 + V2::norm_from_angle(dust_dir as f32) * dust_off as f32;
                    let bubble_part = bubbles.create(Pos::Absolute(pos));
                    bubble_part.image_index = rand.gen_usize_range("frame", 0, 3) as i32;
                    bubble_part.scale = (0.5 + rand.gen_unit("scale") * 0.6) as f32;
                }
            }
            else {
                // Hit by car.
                let corpse = corpses.create(Pos::Absolute(corpse_pos));
                corpse.skin = self.skin.clone();
            }

            visual_effects.screenshake();
            visual_effects.whiteout();
        }

        self.pos.x = x;
        self.pos.y = y;
        self.moving = player_state.moving;
    }
}

impl IsEntity for PlayerLocal {
    fn create(e: Entity) -> Self {
        Self::new(e.id, e.pos.get_abs())
    }

    fn get(&self) -> Entity {
        Entity {
            id: self.entity_id,
            entity_type: EntityType::Player,
            pos: Pos::Absolute(self.pos),
        }
    }

    fn set_pos(&mut self, pos : Pos) {
        if let Pos::Absolute(p) = pos {
            self.pos = p;
        }
    }

    fn get_depth(&self) -> i32 {
        self.pos.y as i32 * 8
    }

    fn draw(&mut self) {
        if (!self.created_corpse) {
            sprites::draw("shadow", 0, self.pos.x * 8.0, self.pos.y * 8.0);
            sprites::draw_with_flip(&self.skin.sprite, self.image_index as usize, self.pos.x * 8.0, self.pos.y * 8.0 - 2.0, self.x_flip);
        }
    }
}