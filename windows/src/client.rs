use crossy_multi_core::{crossy_ruleset::{CrossyRulesetFST, GameConfig, RulesState}, map::RowType, math::V2, ring_buffer::RingBuffer, timeline::{Timeline, TICK_INTERVAL_US}, CoordPos, Input, PlayerId, PlayerInputs, Pos};
use crate::{audio::{self, g_music_volume}, dan_lerp, entities::{self, create_dust, Entity, EntityContainer, EntityManager, OutfitSwitcher, PropController}, gamepad_pressed, hex_color, key_pressed, lerp_color_rgba, pause::{Pause, PauseResult}, player_local::{PlayerInputController, PlayerLocal, Skin}, rope::NodeType, sprites, title_screen::{self, ActorController, TitleScreen}, to_vector2, BLACK, WHITE};
use froggy_rand::FroggyRand;

pub struct Client {
    t: i32,

    pub debug: bool,

    // Disable some gui / teaching elements.
    pub trailer_mode: bool,

    pub exit: bool,
    pub seed: String,

    pub pause: Option<Pause>,

    pub title_screen: Option<TitleScreen>,

    pub timeline: Timeline,
    pub camera: Camera,

    pub prop_controller: PropController,
    pub entities: EntityManager,
    pub visual_effects: VisualEffects,

    pub screen_shader: crate::ScreenShader,

    pub big_text_controller: crate::bigtext::BigTextController,
    pub player_input_controller: PlayerInputController,

    prev_rules: Option<CrossyRulesetFST>,

    actor_controller: ActorController,

    bg_music: TitleBGMusic,

    pub frame_ring_buffer: RingBuffer<Option<Vec<u8>>>,
    pub recording_gif: bool,
    pub recording_gif_name: String,
}

pub const grass_col_0: raylib_sys::Color = hex_color("c4e6b5".as_bytes());
pub const grass_col_1: raylib_sys::Color = hex_color("d1bfdb".as_bytes());
pub const river_col_0: raylib_sys::Color = hex_color("6c6ce2".as_bytes());
pub const river_col_1: raylib_sys::Color = hex_color("5b5be7".as_bytes());
pub const road_col_0: raylib_sys::Color = hex_color("646469".as_bytes());
pub const road_col_1: raylib_sys::Color = hex_color("59595d".as_bytes());
pub const icy_col_0: raylib_sys::Color = hex_color("cbdbfc".as_bytes());
pub const icy_col_1: raylib_sys::Color = hex_color("9badb7".as_bytes());

impl Client {
    pub fn new(debug: bool, seed: &str) -> Self {
        println!("Initialising, Seed {}", seed);
        let mut game_config = GameConfig::default();
        //game_config.bypass_lobby = true;
        //game_config.minimum_players = 1;
        let timeline = Timeline::from_seed(game_config, seed);
        let entities = EntityManager::new();

        let mut actor_controller = ActorController::default();
        //actor_controller.spawn_positions_grid.push((V2::new(20.0, 17.0), false));
        actor_controller.spawn_positions_grid.push((V2::new(0.0, 3.0), true));

        Self {
            t: 0,
            debug,
            trailer_mode: false,
            seed: seed.to_owned(),
            exit: false,
            timeline,
            camera: Camera::new(),
            entities,
            prop_controller: PropController::new(),
            visual_effects: VisualEffects::default(),
            screen_shader: crate::ScreenShader::new(),
            big_text_controller: Default::default(),
            player_input_controller: PlayerInputController::default(),
            prev_rules: Default::default(),
            pause: None,
            title_screen: Some(TitleScreen::default()),
            actor_controller,
            bg_music: TitleBGMusic::new(),
            frame_ring_buffer: RingBuffer::new_with_value(60 * 60, None),
            recording_gif: false,
            recording_gif_name: String::default(),
        }
    }

    pub fn goto_loby_seed(&mut self, seed: &str, bypass_lobby: Option<bool>) {
        let mut config = self.timeline.top_state().rules_state.config.clone();
        if let Some(bl) =  bypass_lobby {
            config.bypass_lobby = bl;
        }

        if (!seed.is_empty()) {
            self.seed = seed.to_owned();
        }

        self.timeline = Timeline::from_seed(config, &self.seed);

        self.player_input_controller = PlayerInputController::default();
        self.entities.clear_round_entities();
        self.entities.players.inner.clear();

        self.pause = None;

        self.visual_effects.noise();
        self.visual_effects.whiteout();
        self.visual_effects.screenshake();
        audio::play("car");
    }

    pub fn tick(&mut self) {
        self.bg_music.tick();
        self.visual_effects.tick();

        if let Some(pause) = self.pause.as_mut() {
            match pause.tick(&mut self.visual_effects) {
                PauseResult::Nothing => {},
                PauseResult::Unpause => {
                    self.pause = None;
                },
                PauseResult::Exit => {
                    self.exit = true;
                }
                PauseResult::Lobby => {
                    self.goto_loby_seed(&crate::shitty_rand_seed(), Some(false));
                },
                PauseResult::Feedback => {
                    // @TODO
                    self.pause = None;
                },
            }

            return;
        }

        self.t += 1;

        let mut just_left_title = false;
        if let Some(title) = self.title_screen.as_mut() {
            if (title.t - title.goto_next_t.unwrap_or(title.t) > 10) {
                self.bg_music.mode = BGMusicMode::FadingOutLowpass;
            }
            else {
                self.bg_music.mode = BGMusicMode::Lowpassed;
            }

            if !title.tick(&mut self.visual_effects, self.bg_music.current_time_in_secs()) {
                self.title_screen = None;
                just_left_title = true;
            }
            else {
                // @Hacky
                self.camera.k = 100.0;
                self.camera.y = -200.0;
                self.camera.y_mod = -200.0;
                self.camera.target_y = -200.0;
                return;
            }
        }
        else {
            if let CrossyRulesetFST::Lobby { .. } = &self.timeline.top_state().rules_state.fst {
                self.bg_music.mode = BGMusicMode::Normal;
            }
            else {
                self.bg_music.mode = BGMusicMode::Paused;
            }
        }

        let (inputs, new_players) = self.player_input_controller.tick(&mut self.timeline, &mut self.entities.players, &self.entities.outfit_switchers);
        self.timeline.tick(Some(inputs), TICK_INTERVAL_US);

        let transitions = {
            let top = self.timeline.top_state();
            StateTransition::new(&top.rules_state.fst, &self.prev_rules)
        };

        //if (transitions.into_round) {
            //self.visual_effects.whiteout();
        //}

        if (transitions.into_lobby && !just_left_title) {
            self.visual_effects.noise();
            self.visual_effects.whiteout();
            self.visual_effects.screenshake();
        }

        if (transitions.leaving_lobby) {
            self.visual_effects.noise();
            self.visual_effects.whiteout();
            self.visual_effects.screenshake();
            audio::play("car");
        }

        if (transitions.into_round_warmup) {
            self.visual_effects.noise();
        }

        if (!new_players.is_empty())
        {
            audio::play("join");
            audio::play("car");
            self.visual_effects.whiteout();
            self.visual_effects.screenshake();

            for new in new_players.iter() {
                if let Some(local) = self.entities.players.inner.iter().find(|x| x.player_id == *new) {
                    self.visual_effects.set_gamepad_vibration(local.controller_id, local.steam_controller_id);
                }
            }
        }

        self.camera.tick(Some(self.timeline.top_state().get_rule_state()), &self.visual_effects, &transitions);

        let top = self.timeline.top_state();
        let mut to_remove = Vec::new();
        for local_player in self.entities.players.inner.iter_mut() {
            if let Some(state) = top.player_states.get(local_player.player_id) {
                let player_state = state.to_public(top.get_round_id(), top.time_us, &self.timeline.map, &top.rules_state.fst);
                let alive_state = top.rules_state.fst.get_player_alive(local_player.player_id);
                local_player.tick(
                    &player_state,
                    alive_state,
                    &self.timeline,
                    &mut self.visual_effects,
                    &mut self.big_text_controller,
                    &mut self.entities.dust,
                    &mut self.entities.bubbles,
                    &mut self.entities.corpses,
                    &mut self.entities.crowns,
                    &mut self.entities.outfit_switchers);
            }
            else {
                // Remove the player
                local_player.kill_animation(&mut self.visual_effects, None, &self.timeline, &mut self.entities.corpses, &mut self.entities.bubbles);
                to_remove.push((local_player.player_id, local_player.entity_id));
            }
        }

        for (remove_player_id, remove_entity_id) in to_remove {
            self.player_input_controller.remove(remove_player_id);
            self.entities.players.delete_entity_id(remove_entity_id);
        }

        if (transitions.into_round_warmup)
        {
            for player in self.entities.players.inner.iter_mut() {
                player.reset();
            }
        }

        if (transitions.into_lobby) {
            for player in self.entities.players.inner.iter_mut() {
                player.reset();
            }
        }

        self.prop_controller.tick(
            &top.rules_state,
            &self.timeline.map,
            &mut self.entities,
            &transitions,
            self.camera.y as i32 / 8);

        if let CrossyRulesetFST::Lobby { .. } = &top.rules_state.fst {
            let rand = FroggyRand::from_hash((self.timeline.map.get_seed(), top.rules_state.fst.get_round_id(), top.rules_state.game_id, self.prop_controller.t));
            create_outfit_switchers(rand, &self.timeline, &self.entities.players, &mut self.entities.outfit_switchers);

            for switcher in self.entities.outfit_switchers.inner.iter() {
                let rand = rand.subrand(switcher.pos);
                if (rand.gen_unit(1) < 0.4) {
                    let dust = create_dust(rand, &mut self.entities.dust, 4.0, 6.0, V2::new(switcher.pos.x as f32 * 8.0 + 4.0, switcher.pos.y as f32 * 8.0 + 4.0));
                    dust.tint = (Skin::from_enum(switcher.skin).color);
                }
            }
        }

        // Handle crowd sounds.
        match &top.rules_state.fst {
            CrossyRulesetFST::RoundWarmup(_) | CrossyRulesetFST::Round(_) | CrossyRulesetFST::RoundCooldown(_)
            => {
                let screen_offset = (self.camera.y.min(0.0)).abs();
                audio::ensure_playing_with_volume("win", 1.0 / (1.0 + 0.1 * screen_offset));
            },
            _ => {
                audio::stop("win");
            },
        }

        // @TODO how do we model this?
        // Should cars be ephemeral actors?
        self.entities.cars.inner.clear();
        self.entities.lillipads.inner.clear();

        //let rows = self.timeline.map.get_row_view(top.get_round_id(), top.rules_state.fst.get_screen_y());
        let pub_cars = self.timeline.map.get_cars(top.get_round_id(), top.time_us);
        for pub_car in pub_cars {
            let car_id = self.entities.create_entity(Entity {
                id: 0,
                entity_type: entities::EntityType::Car,
                pos: Pos::Absolute(V2::new(pub_car.0 as f32 * 8.0, pub_car.1 as f32 * 8.0)),
            });
            let car = self.entities.cars.get_mut(car_id).unwrap();
            car.flipped = pub_car.2;
        }

        let pub_lillies = self.timeline.map.get_lillipads(top.get_round_id(), top.time_us);
        for pub_lilly in pub_lillies {
            let lilly_id = self.entities.create_entity(Entity {
                id: 0,
                entity_type: entities::EntityType::Lillipad,
                pos: Pos::Absolute(V2::new(pub_lilly.0 as f32 * 8.0, pub_lilly.1 as f32 * 8.0)),
            });
            let lilly = self.entities.lillipads.get_mut(lilly_id).unwrap();
        }

        if let CrossyRulesetFST::Lobby { raft_pos, .. } = &top.rules_state.fst {
            let pos = V2::new(*raft_pos, 10.0) * 8.0;

            if self.entities.raft_sails.inner.is_empty() {
                let raft = self.entities.raft_sails.create(Pos::Absolute(pos));
                raft.setup();
            }

            let raft = self.entities.raft_sails.inner.first_mut().unwrap();
            raft.tick(pos);
        }
        else {
            self.entities.raft_sails.inner.clear();
        }

        self.big_text_controller.tick(&self.timeline, &self.entities.players, &transitions, &new_players, self.camera.y);

        let camera_y_max = top.rules_state.fst.get_screen_y() as f32 + 200.0;
        self.entities.bubbles.prune_dead(camera_y_max);
        self.entities.props.prune_dead(camera_y_max);
        self.entities.dust.prune_dead(camera_y_max);
        self.entities.crowns.prune_dead(camera_y_max);
        self.entities.snowflakes.prune_dead(camera_y_max);

        self.prev_rules = Some(top.rules_state.clone().fst);

        if let CrossyRulesetFST::Lobby { .. } = &top.rules_state.fst {
            self.actor_controller.tick(self.bg_music.current_time_in_secs());
        }
        else {
            self.actor_controller.reset();
        }
    }

    pub unsafe fn draw(&mut self) {
        let top = self.timeline.top_state();

        raylib_sys::BeginMode2D(self.camera.to_raylib());

        //const bg_fill_col: raylib_sys::Color = hex_color("3c285d".as_bytes());
        raylib_sys::ClearBackground(BLACK);

        //let draw_bg_tiles = self.title_screen.as_ref().map(|x| x.draw_bg_tiles).unwrap_or(true);
        let draw_bg_tiles = true;

        if (draw_bg_tiles)
        {
            // Draw background
            //let screen_y = top.rules_state.fst.get_screen_y();
            let screen_y = self.camera.y as i32 / 8;
            let round_id = top.get_round_id();
            let rows = self.timeline.map.get_row_view(round_id, screen_y);

            for row_with_y in rows {
                let row = row_with_y.row;
                let y = row_with_y.y;

                let (col_0, col_1) = match row.row_type {
                    RowType::River(_) | RowType::LobbyRiver => {
                        (river_col_0, river_col_1)
                    },
                    RowType::Road(_) => {
                        (road_col_0, road_col_1)
                    },
                    RowType::IcyRow{..} => {
                        (icy_col_0, icy_col_1)
                    },
                    RowType::Lobby => {
                        let t = if y > 0 {
                            //println!("y = {} t = 0", y);
                            0.0
                        }
                        else {
                            let yy = -y as f32;
                            let t = (yy as f32 / 6.0).clamp(0.0, 1.0);
                            //println!("y = {} yy = {} t = {}", y, yy, t);
                            t
                        };

                        //let t = (-(y as f32).min(0.0) / 10.0).clamp(0.0, 1.0);
                        (lerp_color_rgba(grass_col_0, BLACK, t), lerp_color_rgba(grass_col_1, BLACK, t))
                    }
                    _ => {
                        (grass_col_0, grass_col_1)
                    },
                };

                for x in (0..160 / 8) {
                    let col = if (x + y) % 2 == 0 {
                        col_0
                    }
                    else {
                        col_1
                    };

                    raylib_sys::DrawRectangle(x * 8, y * 8, 8, 8, col);
                }

                if let RowType::Bushes(bush_descr) = &row.row_type {
                    for i in 0..=bush_descr.path_descr.wall_width {
                        sprites::draw("tree_top", 1, i as f32 * 8.0, y as f32 * 8.0);
                        sprites::draw("tree_top", 1, (19 - i) as f32 * 8.0, y as f32 * 8.0);
                    }
                    //let hydrated = bush_descr.hydrate();
                }

                if let RowType::LobbyRiver = &row.row_type {
                    if let CrossyRulesetFST::Lobby { raft_pos, .. } = &top.rules_state.fst {
                        for i in 0..4 {
                            sprites::draw("log", 0, (*raft_pos as f32 + i as f32) * 8.0, y as f32 * 8.0);
                        }
                    }
                }

                if let RowType::LobbyRiverBankLower = &row.row_type {
                    for i in 0..20 {
                        sprites::draw("tree_top", 1, i as f32 * 8.0, y as f32 * 8.0);
                    }
                }

                if let RowType::LobbyMain = &row.row_type {
                    let i = 1;
                    sprites::draw("tree_top", 1, i as f32 * 8.0, y as f32 * 8.0);
                    let i = 18;
                    sprites::draw("tree_top", 1, i as f32 * 8.0, y as f32 * 8.0);
                }

                if let RowType::IcyRow(state) = &row.row_type {
                    //for i in 0..=state.path_descr.wall_width {
                    //    sprites::draw("tree_top", 1, i as f32 * 8.0, y as f32 * 8.0);
                    //    sprites::draw("tree_top", 1, (19 - i) as f32 * 8.0, y as f32 * 8.0);
                    //}

                    for x in 0..20 {
                        if x <= state.path_descr.wall_width || x >= 19 - state.path_descr.wall_width || state.blocks.get(x as i32) {
                            sprites::draw("tree_top", 1, x as f32 * 8.0, y as f32 * 8.0);
                        }
                    }
                    //for block in hydrated.blocks {
                    //    sprites::draw("tree_top", 1, block as f32 * 8.0, y as f32 * 8.0);
                    //}
                    //for ice in hydrated.ice {
                    //    sprites::draw("tree_top", 0, ice as f32 * 8.0, y as f32 * 8.0);
                    //}
                }

                if let RowType::Path { wall_width } = row.row_type {
                    for i in 0..=wall_width {
                        sprites::draw("tree_top", 1, i as f32 * 8.0, y as f32 * 8.0);
                        sprites::draw("tree_top", 1, (19 - i) as f32 * 8.0, y as f32 * 8.0);
                    }
                }

                if let RowType::Stands = row.row_type {
                    sprites::draw("block", 0, 6.0 * 8.0, y as f32 * 8.0);
                    sprites::draw("block", 0, (19.0 - 6.0) * 8.0, y as f32 * 8.0);
                }

                if let RowType::StartingBarrier = row.row_type {
                    for i in 0..=6 {
                        sprites::draw("block", 0, i as f32 * 8.0, y as f32 * 8.0);
                        sprites::draw("block", 0, (19.0 - i as f32) * 8.0, y as f32 * 8.0);
                    }

                    if let CrossyRulesetFST::RoundWarmup(_) = &top.rules_state.fst {
                        for i in 7..(20-7) {
                            sprites::draw("barrier", 0, i as f32 * 8.0, y as f32 * 8.0);
                        }
                    }
                }
            }
        }

        if let CrossyRulesetFST::Lobby { raft_pos, .. } = &top.rules_state.fst {
            let players_in_ready_zone = top.player_states.iter().filter(|(_, x)| crossy_multi_core::crossy_ruleset::player_in_lobby_ready_zone(x)).count();
            let total_player_count = top.player_states.count_populated();

            if (!self.trailer_mode && total_player_count >= top.rules_state.config.minimum_players as usize)
            {
                let pos = V2::new(*raft_pos, 10.0) * 8.0 + V2::new(1.0, 6.0) * 8.0;
                let image_index = players_in_ready_zone + 1;
                if (image_index > 9) {
                    // error aahhhh
                    // @Todo cap number of players
                }
                else {
                    sprites::draw("font_linsenn_m5x7_numbers", image_index, pos.x, pos.y);
                }
                let pos = pos + V2::new(6.0, 0.0);
                sprites::draw("font_linsenn_m5x7_numbers", 0, pos.x, pos.y);
                let pos = pos + V2::new(6.0, 0.0);
                let image_index = total_player_count + 1;
                sprites::draw("font_linsenn_m5x7_numbers", image_index, pos.x, pos.y);
            }
        }

        self.big_text_controller.draw_lower();
        self.actor_controller.draw();

        {
            // @Perf keep some list and insertion sort
            let mut all_entities = Vec::new();
            self.entities.extend_all_depth(&mut all_entities);

            all_entities.sort_by_key(|(_, depth)| *depth);

            for (e, _) in all_entities {
                self.entities.draw_entity(e, self.pause.is_some());
            }
        }

        raylib_sys::EndMode2D();

        if (self.entities.players.inner.len() == 0)
        {
            let bpos = V2::new(60.0, 60.0);
            let pos = bpos - V2::new(20.0, 0.0) + V2::norm_from_angle(self.t as f32 * 0.1);
            sprites::draw_p("keys_arrows", 0, pos);
            let pos = bpos + V2::new(20.0, 0.0) + V2::norm_from_angle(self.t as f32 * 0.1 + 3.141);
            sprites::draw_p("keys_wasd", 0, pos);
            let pos = bpos + V2::new(00.0, 20.0) + V2::norm_from_angle(self.t as f32 * 0.1 + 3.141 * 0.6);
            sprites::draw_p("keys_gamepad", 0, pos);
        }

        if let Some(title) = self.title_screen.as_mut() {
            title.draw();
        }

        self.big_text_controller.draw();

        {
            let settings = crate::settings::get();
            if (settings.flashing && self.visual_effects.whiteout > 0) {
                raylib_sys::DrawRectangle(0, 0, 160, 160, WHITE);
            }
        }

        if let Some(pause) = self.pause.as_mut() {
            pause.draw();
        }
    }
}

pub struct Camera {
    x: f32,
    y: f32,
    x_mod: f32,
    y_mod: f32,
    target_y: f32,
    t: i32,
    k: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            x_mod: 0.0,
            y_mod: 0.0,
            target_y: 0.0,
            t: 0,
            k: 3.0,
        }
    }

    pub fn tick(&mut self, m_rules_state: Option<&RulesState>, visual_effects: &VisualEffects, transitions: &StateTransition) {
        self.t += 1;

        if let Some(rules_state) = m_rules_state {
            self.target_y = match &rules_state.fst {
                CrossyRulesetFST::RoundWarmup(state) => {
                    let remaining_s = state.remaining_us as f32 / 1_000_000.0;
                    let t = ((remaining_s - 3.0) / 3.0).max(0.0);
                    -16.0 * (t * t) * 2.5
                },
                CrossyRulesetFST::Round(round_state) => {
                    round_state.screen_y as f32
                },
                CrossyRulesetFST::RoundCooldown(round_state) => {
                    round_state.round_state.screen_y as f32
                },
                _ => 0.0
            };

            self.k = match &rules_state.fst {
                CrossyRulesetFST::Lobby{ .. } => {
                    // Lerp towards 3
                    dan_lerp(self.k, 3.0, 10.0)
                },
                _ => 3.0
            };
        }

        self.x = 0.0;

        if transitions.into_round {
            self.y = self.target_y * 8.0
        }
        else {
            self.y = dan_lerp(self.y, self.target_y * 8.0, self.k);
        }

        self.x_mod = self.x;
        self.y_mod = self.y;

        let settings = crate::settings::get();
        if (settings.screenshake && visual_effects.screenshake > 0.01) {
            //self.screen_shake_t -= 1.0;
            //let dir = *FroggyRand::new(self.t as u64).choose((), &[-1.0, 1.0]) as f32;
            //self.x = 1.0 / (visual_effects.screenshake + 1.0) * dir;

            let dir = (FroggyRand::new(self.t as u64).gen_unit(0) * 3.141 * 2.0) as f32;
            let mag = visual_effects.screenshake * 0.4;
            let offset = V2::norm_from_angle(dir) * mag;
            self.x_mod = self.x + offset.x;
            self.y_mod = self.y + offset.y;
        }
    }

    pub fn to_raylib(&self) -> raylib_sys::Camera2D {
        raylib_sys::Camera2D {
            offset: raylib_sys::Vector2::zero(),
            target: raylib_sys::Vector2 { x: self.x_mod, y: self.y_mod },
            rotation: 0.0,
            zoom: 1.0,

        }
    }
}

pub struct VisualEffects {
    pub t: i32,
    pub whiteout: i32,
    pub screenshake: f32,
    pub noise: f32,

    pub controller_vibrations: Vec<f32>,

    #[cfg(feature = "steam")]
    pub steam_controller_vibrations: crate::steam::SteamControllerMap<f32>,
}

impl Default for VisualEffects {
    fn default() -> Self {
        let mut vibration = Vec::new();
        for i in 0..4 {
            vibration.push(0.0);
        }

        Self {
            t: 0,
            whiteout: 0,
            screenshake: 0.0,
            noise: 0.0,
            controller_vibrations: vibration,

            #[cfg(feature = "steam")]
            steam_controller_vibrations: Default::default(),
        }
    }
}

impl VisualEffects {
    pub fn whiteout(&mut self) {
        self.whiteout = self.whiteout.max(6);
    }

    pub fn screenshake(&mut self) {
        self.screenshake = self.screenshake.max(15.0);
        self.noise = self.noise.max(15.0);
    }

    pub fn noise(&mut self) {
        self.noise = self.noise.max(15.0);
    }

    pub fn set_gamepad_vibration(&mut self, m_controller_id: Option<i32>, m_steam_controller_id: Option<u64>)
    {
        if let Some(controller_id) = m_controller_id {
            self.controller_vibrations[controller_id as usize] = 15.0;
        }

        #[cfg(feature = "steam")]
        if let Some(steam_controller_id) = m_steam_controller_id {
            if let Some(i) = self.steam_controller_vibrations.find(steam_controller_id) {
                self.steam_controller_vibrations.inner[i].1 = Some(15.0);
            }
            else {
                if let Some(i) = self.steam_controller_vibrations.find_next_free() {
                    self.steam_controller_vibrations.inner[i] = (steam_controller_id, Some(15.0));
                }
                else {
                    // Will we ever hit this?
                    debug_assert!(false);
                }
            }
        }
    }

    pub fn tick(&mut self) {
        self.t += 1;

        self.whiteout = (self.whiteout - 1).max(0);
        self.screenshake *= 0.85;
        self.noise *= 0.85;

        let settings = crate::settings::get();
        if (settings.vibration) {
            const MULT: f32 = 0.65;

            for (i, x) in self.controller_vibrations.iter_mut().enumerate() {
                *x *= MULT;

                let id = i as i32;
                #[cfg(target_os="windows")]
                unsafe {
                    if raylib_sys::IsGamepadAvailable(id) {
                        let value = get_vibration_speed(*x);

                        // Lifted from
                        //https://github.com/machlibs/rumble/blob/main/src/up_rumble.h
                        // Call win32 directly
                        let x = windows_sys::Win32::UI::Input::XboxController::XINPUT_VIBRATION {
                            wLeftMotorSpeed: value,
                            wRightMotorSpeed: value,
                        };
                        windows_sys::Win32::UI::Input::XboxController::XInputSetState(id as u32, std::ptr::from_ref(&x));
                    }
                }
            }

            #[cfg(feature = "steam")]
            for ((steam_controller_id, m_vibration)) in self.steam_controller_vibrations.inner.iter_mut() {
                if let Some(x) = m_vibration {
                    *x *= MULT;
                    let value = get_vibration_speed(*x);
                    crate::steam::set_vibration(*steam_controller_id, value);
                }
            }
        }
    }
}

fn get_vibration_speed(x: f32) -> u16 {
    let clamped = x.clamp(0.0, 1.0);

    if clamped < 0.01 {
        return 0;
    }

    (clamped * u16::MAX as f32).floor() as u16
}

#[derive(Default)]
pub struct StateTransition {
    pub into_lobby: bool,
    pub into_round_warmup: bool,
    pub into_round: bool,
    pub into_round_cooldown: bool,
    pub into_winner: bool,

    pub leaving_lobby: bool,
}

impl StateTransition {
    pub fn new(current: &CrossyRulesetFST, prev: &Option<CrossyRulesetFST>) -> Self {
        let mut transitions = Self::default();
        transitions.into_lobby =
            matches!(current, CrossyRulesetFST::Lobby { .. })
            && !matches!(prev, Some(CrossyRulesetFST::Lobby { .. }));
        transitions.leaving_lobby =
            !matches!(current, CrossyRulesetFST::Lobby { .. })
            && matches!(prev, Some(CrossyRulesetFST::Lobby { .. }));

        transitions.into_round_warmup =
            matches!(current, CrossyRulesetFST::RoundWarmup { .. })
            && !matches!(prev, Some(CrossyRulesetFST::RoundWarmup { .. }));
        transitions.into_round =
            matches!(current, CrossyRulesetFST::Round { .. })
            && !matches!(prev, Some(CrossyRulesetFST::Round { .. }));
        transitions.into_round_cooldown =
            matches!(current, CrossyRulesetFST::RoundCooldown { .. })
            && !matches!(prev, Some(CrossyRulesetFST::RoundCooldown { .. }));
        transitions.into_winner =
            matches!(current, CrossyRulesetFST::EndWinner { .. })
            && !matches!(prev, Some(CrossyRulesetFST::EndWinner { .. }));


        transitions
    }
}

fn create_outfit_switchers(rand: FroggyRand, timeline: &Timeline, players: &EntityContainer<PlayerLocal>, outfit_switchers: &mut EntityContainer<OutfitSwitcher>) {
    if (players.inner.len() == 0) {
        return;
    }

    let to_create = 4 - outfit_switchers.inner.len();

    if to_create == 0 {
        return;
    }

    if (rand.gen_unit(()) < 0.998) {
        return;
    }

    let mut options = Vec::new();
    // Not very efficient but doesnt need to be.
    for x in 3..16 {
        for y in 5..9 {
            options.push(CoordPos::new(x, y))
        }
    }

    for player in players.inner.iter() {
        // @Buggy
        // Rough conversion to coordpos, may occcaassionally put someone on top of another, but should usually be fine
        if let Some((idx, _)) = options.iter().enumerate().find(|(_, pos)| **pos == CoordPos::new(player.pos.x.round() as i32, player.pos.y.round() as i32)) {
            options.remove(idx);
        }
    }

    rand.shuffle("shuffle", &mut options);

    // @HACK, create one at a time.
    let to_create = 1;

    for (i, pos) in options.iter().take(to_create).enumerate() {
        let skin = Skin::rand_not_overlapping(rand.subrand(i), &players.inner, &outfit_switchers.inner);
        let switcher = outfit_switchers.create(Pos::Coord(*pos));
        switcher.skin = skin.player_skin;
    }
}

#[derive(Debug)]
enum BGMusicMode {
    Lowpassed,
    FadingOutLowpass,
    Normal,
    Paused,
}

struct TitleBGMusic {
    pub music: raylib_sys::Music,
    pub mode: BGMusicMode,

    // Note this should not be relied on, its just to avoid perf issues.
    // repeatedly trying to fetch state.
    pub playing_unsynced: bool,
}

impl TitleBGMusic {
    pub fn new() -> Self {
        let music = unsafe {
            //let music_path = format!("{}/sounds/mus_jump_at_sun_3.mp3", crate::resource_dir());
            //let music_path = format!("{}/sounds/morrislike_6.mp3", crate::resource_dir());
            let music_path = format!("{}/sounds/morrislike_simple.mp3", crate::resource_dir());
            //let music_path = format!("{}/sounds/snd_viper_full.mp3", crate::resource_dir());
            let mut music = raylib_sys::LoadMusicStream(crate::c_str_leaky(&music_path));
            raylib_sys::SetMusicVolume(music, { g_music_volume });
            music.looping = true;
            raylib_sys::AttachAudioStreamProcessor(music.stream, Some(rl_low_pass));
            music
        };

        Self {
            music,
            mode: BGMusicMode::Paused,
            playing_unsynced: false,
        }
    }

    pub fn current_time_in_secs(&self) -> f32 {
        unsafe {
            raylib_sys::GetMusicTimePlayed(self.music)
        }
    }

    pub fn tick(&mut self) {
        unsafe {
            // @Perf
            // Cache
            raylib_sys::SetMusicVolume(self.music, g_music_volume);
        }

        match self.mode {
            BGMusicMode::Lowpassed => {
                unsafe {
                    LP_FREQ = dan_lerp(LP_FREQ, 300.0, 10.0);
                }
            },
            BGMusicMode::FadingOutLowpass => {
                unsafe {
                    LP_FREQ = dan_lerp(LP_FREQ, 50_000.0, 500.0);
                }
            },
            BGMusicMode::Normal => {
                unsafe {
                    LP_FREQ = dan_lerp(LP_FREQ, 50_000.0, 10.0);
                }
            },
            _ => {},
        }

        match self.mode {
            BGMusicMode::Paused => {
                unsafe {
                    if self.playing_unsynced {
                        raylib_sys::PauseMusicStream(self.music);
                        self.playing_unsynced = false;
                    }
                }
            },
            _ => {
                unsafe {
                    if !self.playing_unsynced {
                        raylib_sys::PlayMusicStream(self.music);
                        self.playing_unsynced = true;
                    }
                }
            },
        }

        unsafe {
            raylib_sys::UpdateMusicStream(self.music);
        }
    }
}

static mut LP_DATA: [f32;2] = [0.0, 0.0];
static mut LP_FREQ: f32 = 100.0;

unsafe extern "C" fn rl_low_pass(buffer_void: *mut ::std::os::raw::c_void, frames: ::std::os::raw::c_uint) {
    let cutoff = LP_FREQ / 44100.0; // 70 Hz lowpass filter
    let k = cutoff / (cutoff + 0.1591549431); // RC filter formula

    // Converts the buffer data before using it
    let buffer_raw : *mut f32 = buffer_void.cast();
    let buffer = std::slice::from_raw_parts_mut(buffer_raw, frames as usize * 2);
    for i in 0..(frames as usize) {
        let index = i * 2;

        let l = buffer[index];
        let r = buffer[index+1];

        LP_DATA[0] += k * (l - LP_DATA[0]);
        LP_DATA[1] += k * (r - LP_DATA[1]);
        buffer[index] = LP_DATA[0];
        buffer[index + 1] = LP_DATA[1];
    }
}
