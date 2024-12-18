use crossy_multi_core::{crossy_ruleset::{CrossyRulesetFST, GameConfig, RulesState}, game, map::RowType, math::V2, player::{PlayerState, PlayerStatePublic}, timeline::{Timeline, TICK_INTERVAL_US}, CoordPos, Input, PlayerId, PlayerInputs, Pos};
use crate::{dan_lerp, entities::{self, Entity, EntityContainer, EntityManager, Prop, PropController, Spectator}, hex_color, key_pressed, player_local::{PlayerInputController, Skin}, sprites, BLACK, WHITE};
use froggy_rand::FroggyRand;

pub struct Client {
    pub exit: bool,
    pub timeline: Timeline,
    pub camera: Camera,

    pub prop_controller: PropController,
    pub entities: EntityManager,
    pub visual_effects: VisualEffects,

    pub screen_shader: crate::ScreenShader,

    pub big_text_controller: crate::bigtext::BigTextController,
    pub player_input_controller: PlayerInputController,

    prev_rules: Option<CrossyRulesetFST>,
}

impl Client {
    pub fn new(debug: bool) -> Self {
        let mut game_config = GameConfig::default();
        //game_config.bypass_lobby = true;
        //game_config.minimum_players = 1;
        let timeline = Timeline::from_seed(game_config, "ac");
        let entities = EntityManager::new();

        Self {
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
        }
    }

    pub fn tick(&mut self) {
        let (inputs, new_players) = self.player_input_controller.tick(&mut self.timeline, &mut self.entities.players);
        self.timeline.tick(Some(inputs), TICK_INTERVAL_US);

        let top = self.timeline.top_state();
        let transitions = StateTransition::new(&top.rules_state.fst, &self.prev_rules);

        //if (transitions.into_round) {
            //self.visual_effects.whiteout();
        //}

        if (transitions.into_round_warmup) {
            self.visual_effects.noise();
        }

        if (!new_players.is_empty())
        {
            self.visual_effects.whiteout();
            self.visual_effects.screenshake();
        }

        self.camera.tick(Some(self.timeline.top_state().get_rule_state()), &self.visual_effects, &transitions);
        self.visual_effects.tick();

        for local_player in self.entities.players.inner.iter_mut() {
            if let Some(state) = top.player_states.get(local_player.player_id) {
                let player_state = state.to_public(top.get_round_id(), top.time_us, &self.timeline.map);
                let alive_state = top.rules_state.fst.get_player_alive(local_player.player_id);
                local_player.tick(
                    &player_state,
                    alive_state,
                    &self.timeline,
                    &mut self.visual_effects,
                    &mut self.entities.dust,
                    &mut self.entities.bubbles,
                    &mut self.entities.corpses,
                    &mut self.entities.crowns);
            }
        }

        if (transitions.into_round_warmup)
        {
            for player in self.entities.players.inner.iter_mut() {
                player.reset();
            }
        }

        self.prop_controller.tick(&top.rules_state, &self.timeline.map, &mut self.entities, &transitions);

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

        self.big_text_controller.tick(&self.timeline, &self.entities.players, &transitions, &new_players);

        let camera_y_max = top.rules_state.fst.get_screen_y() as f32 + 200.0;
        self.entities.bubbles.prune_dead(camera_y_max);
        self.entities.props.prune_dead(camera_y_max);
        self.entities.dust.prune_dead(camera_y_max);
        self.entities.crowns.prune_dead(camera_y_max);

        self.prev_rules = Some(top.rules_state.clone().fst);
    }

    pub unsafe fn draw(&mut self) {
        let top = self.timeline.top_state();

        raylib_sys::BeginMode2D(self.camera.to_raylib());

        {
            // Draw background
            const bg_fill_col: raylib_sys::Color = hex_color("3c285d".as_bytes());
            raylib_sys::ClearBackground(bg_fill_col);
            const grass_col_0: raylib_sys::Color = hex_color("c4e6b5".as_bytes());
            const grass_col_1: raylib_sys::Color = hex_color("d1bfdb".as_bytes());
            const river_col_0: raylib_sys::Color = hex_color("6c6ce2".as_bytes());
            const river_col_1: raylib_sys::Color = hex_color("5b5be7".as_bytes());
            const road_col_0: raylib_sys::Color = hex_color("646469".as_bytes());
            const road_col_1: raylib_sys::Color = hex_color("59595d".as_bytes());

            //let screen_y = top.rules_state.fst.get_screen_y();
            let screen_y = self.camera.y;
            let round_id = top.get_round_id();
            let rows = self.timeline.map.get_row_view(round_id, screen_y as i32 / 8);

            for row_with_y in rows {
                let row = row_with_y.row;
                let y = row_with_y.y;

                let (col_0, col_1) = match row.row_type {
                    RowType::River(_) => {
                        (river_col_0, river_col_1)
                    },
                    RowType::Road(_) => {
                        (road_col_0, road_col_1)
                    },
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
                    let hydrated = bush_descr.hydrate();
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

        if let CrossyRulesetFST::Lobby { time_with_all_players_in_ready_zone } = &top.rules_state.fst {
            let x0 = 7.0 * 8.0;
            let y0 = 14.0 * 8.0;
            let w_base = 6.0 * 8.0;
            let h = 4.0 * 8.0;

            unsafe {
                let proportion = *time_with_all_players_in_ready_zone as f32 / 120.0;
                raylib_sys::DrawRectangleRec(raylib_sys::Rectangle {
                    x: x0,
                    y: y0,
                    width: w_base * proportion,
                    height: h,
                }, WHITE);
                raylib_sys::DrawRectangleLinesEx(raylib_sys::Rectangle {
                    x: x0,
                    y: y0,
                    width: w_base,
                    height: h,
                }, 1.0, BLACK);
            }
        }

        {
            // @Perf keep some list and insertion sort
            let mut all_entities = Vec::new();
            self.entities.extend_all_depth(&mut all_entities);

            all_entities.sort_by_key(|(_, depth)| *depth);

            for (e, _) in all_entities {
                self.entities.draw_entity(e);
            }
        }

        raylib_sys::EndMode2D();

        self.big_text_controller.draw();

        {
            if (self.visual_effects.whiteout > 0) {
                raylib_sys::DrawRectangle(0, 0, 160, 160, WHITE);
            }
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
            }
        }

        self.x = 0.0;

        if transitions.into_round {
            self.y = self.target_y * 8.0
        }
        else {
            self.y = dan_lerp(self.y, self.target_y * 8.0, 3.0);
        }

        self.x_mod = self.x;
        self.y_mod = self.y;

        if (visual_effects.screenshake > 0.01) {
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

#[derive(Default)]
pub struct VisualEffects {
    pub whiteout: i32,
    pub screenshake: f32,
    pub noise: f32,
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

    pub fn tick(&mut self) {
        self.whiteout = (self.whiteout - 1).max(0);
        self.screenshake *= 0.85;
        self.noise *= 0.85;
    }
}

#[derive(Default)]
pub struct StateTransition {
    pub into_lobby: bool,
    pub into_round_warmup: bool,
    pub into_round: bool,
    pub into_round_cooldown: bool,
    pub into_end: bool,
}

impl StateTransition {
    pub fn new(current: &CrossyRulesetFST, prev: &Option<CrossyRulesetFST>) -> Self {
        let mut transitions = Self::default();
        transitions.into_lobby = 
            matches!(current, CrossyRulesetFST::Lobby { .. })
            && !matches!(prev, Some(CrossyRulesetFST::Lobby { .. }));
        transitions.into_round_warmup = 
            matches!(current, CrossyRulesetFST::RoundWarmup { .. })
            && !matches!(prev, Some(CrossyRulesetFST::RoundWarmup { .. }));
        transitions.into_round = 
            matches!(current, CrossyRulesetFST::Round { .. })
            && !matches!(prev, Some(CrossyRulesetFST::Round { .. }));
        transitions.into_round_cooldown = 
            matches!(current, CrossyRulesetFST::RoundCooldown { .. })
            && !matches!(prev, Some(CrossyRulesetFST::RoundCooldown { .. }));
        transitions.into_end = 
            matches!(current, CrossyRulesetFST::EndWinner { .. })
            && !matches!(prev, Some(CrossyRulesetFST::EndWinner { .. }));

        transitions
    }
}