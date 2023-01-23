#![allow(unused_parens)]

macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into())
    }
}

mod wasm_instant;
mod ai;

use std::time::Duration;
use std::cell::RefCell;

use std::collections::VecDeque;
use wasm_instant::WasmInstant;
use serde::Deserialize;
use wasm_bindgen::prelude::*;

use crossy_multi_core::*;
use crossy_multi_core::game::PlayerId;
use crossy_multi_core::map::river::RiverSpawnTimes;
use crossy_multi_core::crossy_ruleset::AliveState;

struct ConsoleDebugLogger();
impl crossy_multi_core::DebugLogger for ConsoleDebugLogger {
    fn log(&self, logline: &str) {
        log!("{}", logline);
    }
}

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[derive(Debug)]
pub struct LocalPlayerInfo {
    player_id: game::PlayerId,
    buffered_input : Input,
}

const TIME_REQUEST_INTERVAL : u32 = 13;

#[wasm_bindgen]
#[derive(Debug)]
pub struct Client {
    client_start : WasmInstant,
    server_start: WasmInstant,
    estimated_latency_us : f32,

    timeline: timeline::Timeline,
    last_tick: u32,
    // The last server tick we received
    last_server_tick: Option<u32>,
    last_server_frame_id: Option<u32>,
    local_player_info : Option<LocalPlayerInfo>,
    ready_state : bool,

    // This seems like a super hacky solution
    trusted_rule_state : Option<crossy_ruleset::CrossyRulesetFST>,

    //queued_server_messages : VecDeque<interop::ServerTick>,
    queued_time_info : Option<interop::TimeRequestEnd>,

    queued_server_linden_messages : VecDeque<interop::LindenServerTick>,

    ai_agent : Option<RefCell<Box<dyn ai::AIAgent>>>,
}

#[wasm_bindgen]
impl Client {

    #[wasm_bindgen(constructor)]
    pub fn new(seed : &str, server_frame_id : u32, server_time_us : u32, estimated_latency_us : u32) -> Self {
        // Setup statics
        console_error_panic_hook::set_once();
        crossy_multi_core::set_debug_logger(Box::new(ConsoleDebugLogger()));

        let estimated_current_frame_id = server_frame_id + estimated_latency_us / 16_666;
        let timeline = timeline::Timeline::from_server_parts(seed, estimated_current_frame_id, server_time_us, vec![], crossy_ruleset::CrossyRulesetFST::start());

        // Estimate server start
        let client_start = WasmInstant::now();
        let server_start = client_start - Duration::from_micros((server_time_us + estimated_latency_us) as u64);

        //log!("CONSTRUCTING : Estimated t0 {:?} server t1 {} estimated latency {}", server_start, server_time_us, estimated_latency_us);

        log!("LINDEN CLIENT : estimated latency {}, server frame_id {}, estimated now server_frame_id {}", estimated_latency_us, server_frame_id, estimated_current_frame_id);


        Client {
            timeline,
            last_tick : server_time_us,
            last_server_tick : None,
            last_server_frame_id: None,
            client_start,
            server_start,
            estimated_latency_us : estimated_latency_us as f32,
            local_player_info : None,
            // TODO proper ready state
            ready_state : false,
            trusted_rule_state: None,
            //queued_server_messages: Default::default(),
            queued_time_info: Default::default(),
            queued_server_linden_messages: Default::default(),
            ai_agent : None,
        } 
    }

    pub fn join(&mut self, player_id : u32) {
        self.local_player_info = Some(LocalPlayerInfo {
            player_id : PlayerId(player_id as u8),
            buffered_input : Input::None,
        })
    }

    pub fn get_ready_state(&self) -> bool {
        self.ready_state
    }

    pub fn set_ready_state(&mut self, state : bool) {
        self.ready_state = state;
    }

    pub fn buffer_input_json(&mut self, input_json : &str) {
        let input = serde_json::from_str(input_json).map_err(|e| log!("{} {:?}", input_json, e)).unwrap_or(Input::None);
        self.buffer_input(input);
    }

    fn buffer_input(&mut self, input : Input) {
        self.local_player_info.as_mut().map(|x| {
            //if input != Input::None {
            if input != Input::None && x.buffered_input == Input::None {
                x.buffered_input = input;
            }
        });
    }

    pub fn get_top_frame_id(&self) -> u32 {
        self.timeline.top_state().frame_id
    }

    pub fn tick(&mut self) {
        const TICK_INTERVAL_US : u32 = 16_666;

        let current_time = self.server_start.elapsed();
        let current_time_us = current_time.as_micros() as u32;

        loop {
            let delta_time = current_time_us.saturating_sub(self.last_tick);
            if (delta_time > TICK_INTERVAL_US)
            {
                let tick_time = self.last_tick + TICK_INTERVAL_US;
                self.tick_inner(tick_time);
                self.last_tick = tick_time;
            }
            else
            {
                break;
            }
        }
    }

    pub fn tick_inner(&mut self, current_time_us : u32) {
        //let current_time = self.server_start.elapsed();
        //self.last_tick = current_time.as_micros() as u32;

        // Move buffered input to input
        // awkward because of mut / immut borrowing
        let mut player_inputs = self.timeline.get_last_player_inputs();

        let mut can_move = false;
        let local_player_id = self.local_player_info.as_ref().map(|x| x.player_id);
        local_player_id.map(|id| {
            self.timeline.top_state().get_player(id).map(|player| {
                can_move = player.can_move();
            });
        });

        if (self.local_player_info.is_some())
        {
            let mut local_input = Input::None;

            if (can_move)
            {
                if let Some(ai_refcell) = &self.ai_agent {
                    let mut ai = ai_refcell.borrow_mut();
                    local_input = ai.think(&self.timeline.top_state(), &self.timeline.map);
                }
                else 
                {
                    let local_player_info = self.local_player_info.as_mut().unwrap();
                    if (local_player_info.buffered_input != Input::None)
                    {
                        local_input = local_player_info.buffered_input;
                        local_player_info.buffered_input = Input::None;
                    }
                }
            }

            player_inputs.set(self.local_player_info.as_ref().unwrap().player_id, local_input);
        }

        // Tick 
        if (self.timeline.top_state().time_us > current_time_us)
        {
            log!("OH NO WE ARE IN THE PAST!");
        }
        else
        {
            self.timeline
                .tick_current_time(Some(player_inputs), current_time_us);
        }

        /*
        // BIGGEST hack
        // dont have the energy to explain, but the timing is fucked and just want to demo something.
        let mut server_tick_it = None;
        while  {
            self.queued_server_messages.back().map(|x| x.latest.time_us < current_time_us).unwrap_or(false)
        }
        {server_tick_it = self.queued_server_messages.pop_back();}

        self.process_time_info();

        if let Some(server_tick) = server_tick_it {
            self.process_server_message(&server_tick);
        }
        */

        let current_frame_id = self.timeline.top_state().frame_id;
        let mut server_tick_it = None;
        while  {
            self.queued_server_linden_messages.back().map(|x| x.latest.frame_id < current_frame_id).unwrap_or(false)
        }
        {server_tick_it = self.queued_server_linden_messages.pop_back();}

        self.process_time_info();

        if let Some(linden_server_tick) = server_tick_it {
            self.process_linden_server_message(&linden_server_tick);
        }

        //if (self.timeline.top_state().frame_id.floor() as u32 % 15) == 0
        {
            //log!("{:?}", self.timeline.top_state().get_rule_state());
            //log!("{:?}", self.timeline.top_state());
        }
    }

    fn process_time_info(&mut self)
    {
        if let Some(time_request_end) = self.queued_time_info.take() {
            let t0 = time_request_end.client_send_time_us as i64;
            let t1 = time_request_end.server_receive_time_us as i64;
            let t2 = time_request_end.server_send_time_us as i64;
            let t3 = time_request_end.client_receive_time_us as i64;

            let total_time_in_flight = t3 - t0;
            let total_time_on_server = t2 - t1;
            let ed = (total_time_in_flight - total_time_on_server) / 2;

            let latency_lerp_k = 50. / TIME_REQUEST_INTERVAL as f32;

            self.estimated_latency_us = dan_lerp(self.estimated_latency_us, ed as f32, latency_lerp_k);

            let time_now_us = WasmInstant::now().saturating_duration_since(self.client_start).as_micros() as u32;
            let estimated_server_time_us = t2 as u32 + self.estimated_latency_us as u32;

            let holding_time = time_now_us - t3 as u32;
            //log!("Holding time {}us", holding_time);

            let new_server_start = self.client_start + Duration::from_micros(t3 as u64 + holding_time as u64) - Duration::from_micros(estimated_server_time_us as u64);

            let server_start_lerp_k_up = 500. / TIME_REQUEST_INTERVAL as f32;
            let server_start_lerp_k_down = 500. / TIME_REQUEST_INTERVAL as f32;
            self.server_start = WasmInstant(dan_lerp_directional(self.server_start.0 as f32, new_server_start.0 as f32, server_start_lerp_k_up, server_start_lerp_k_down) as i128);
            //log!("estimated latency {}ms", self.estimated_latency_us as f32 / 1000.);
            //log!("estimated server start {}delta_ms", self.server_start.0 as f32 / 1000.);
        }
    }

    /*
    fn process_server_message(&mut self, server_tick : &interop::ServerTick)
    {
        // If we have had a "major change" instead of patching up the current state we perform a full reset
        // At the moment a major change is either:
        //   We have moved between game states (eg the round ended)
        //   A player has joined or left
        let mut should_reset = self.trusted_rule_state.as_ref().map(|x| !x.same_variant(&server_tick.rule_state)).unwrap_or(false);
        should_reset |= self.timeline.top_state().player_states.count_populated() != server_tick.latest.states.len();

        if (should_reset) {
            self.timeline = timeline::Timeline::from_server_parts_exact_seed(
                self.timeline.map.get_seed(),
                server_tick.latest.time_us,
                server_tick.latest.states.clone(),
                server_tick.rule_state.clone());

            // Reset ready state when we are not in the lobby
            match (server_tick.rule_state)
            {
                crossy_ruleset::CrossyRulesetFST::Lobby(_) => {},
                _ => {self.ready_state = false}
            }
        }
        else
        {
            match self.local_player_info.as_ref()
            {
                Some(lpi) => {
                    if (self.timeline.top_state().get_player(lpi.player_id)).is_none()
                    {
                        // Edge case
                        // First tick with the player
                        // we need to take state from server
                        self.timeline.propagate_state(
                            &server_tick.latest,
                            Some(&server_tick.rule_state),
                            Some(&server_tick.latest),
                            None);
                    }
                    else
                    {
                        self.timeline.propagate_state(
                            &server_tick.latest,
                            Some(&server_tick.rule_state),
                            server_tick.last_client_sent.get(lpi.player_id),
                            Some(lpi.player_id));
                    }
                }
                _ => {
                    self.timeline.propagate_state(
                        &server_tick.latest,
                        Some(&server_tick.rule_state),
                        None,
                        None);
                }
            }
        }

        self.last_server_tick = Some(server_tick.latest.time_us);
        self.trusted_rule_state = Some(server_tick.rule_state.clone());
    }
    */

    fn process_linden_server_message(&mut self, linden_server_tick : &interop::LindenServerTick)
    {
        //let mut should_reset = self.trusted_rule_state.as_ref().map(|x| !x.same_variant(&linden_server_tick.rule_state)).unwrap_or(false);
        //should_reset |= self.timeline.top_state().player_states.count_populated() != linden_server_tick.latest.states.len();

        let should_reset = false;

        if (should_reset)
        {
            self.timeline = timeline::Timeline::from_server_parts_exact_seed(
                self.timeline.map.get_seed(),
                linden_server_tick.latest.frame_id,
                linden_server_tick.latest.time_us,
                linden_server_tick.latest.states.clone(),
                linden_server_tick.rule_state.clone());

            // Reset ready state when we are not in the lobby
            match (linden_server_tick.rule_state)
            {
                crossy_ruleset::CrossyRulesetFST::Lobby(_) => {},
                _ => {self.ready_state = false}
            }
        }
        else
        {
            if let Some(client_state_at_lkg_time) = (self.timeline.try_get_state(linden_server_tick.lkg_state.frame_id))
            {
                if (linden_server_tick.lkg_state.player_states != client_state_at_lkg_time.player_states)
                {
                    log!("Mismatch in LKG! frame_id {}", client_state_at_lkg_time.frame_id);
                    log!("Local at lkg time {:#?}", client_state_at_lkg_time.player_states);
                    log!("LKG {:#?}", linden_server_tick.lkg_state.player_states);

                    // TODO We do a ton of extra work, we recalculate from lkg with current inputs then run propate inputs from server.
                    self.timeline = self.timeline.rebase(&linden_server_tick.lkg_state);
                    //log!("Local {:#?}", client_state_at_lkg_time.player_states);
                    //log!("Remote {:#?}", linden_server_tick.lkg_state);
                    //self.timeline.states
                }
            }
            //log!("Propagating inputs {:#?}", linden_server_tick.delta_inputs);
            self.timeline.propagate_inputs(linden_server_tick.delta_inputs.clone());
        }

        //self.last_server_tick = Some(linden_server_tick.latest.time_us);
        self.last_server_frame_id = Some(linden_server_tick.latest.frame_id);
        self.trusted_rule_state = Some(linden_server_tick.rule_state.clone());
    }

    fn get_round_id(&self) -> u8 {
        self.trusted_rule_state.as_ref().map(|x| x.get_round_id()).unwrap_or(0)
    }

    fn get_river_spawn_times(&self) -> &RiverSpawnTimes {
        self.trusted_rule_state.as_ref().map(|x| x.get_river_spawn_times()).unwrap_or(&crossy_multi_core::map::river::EMPTY_RIVER_SPAWN_TIMES)
    }

    pub fn recv(&mut self, server_tick : &[u8])
    {
        if let Some(deserialized) = try_deserialize_message(server_tick)
        {
            self.recv_internal(deserialized);
        }
    }

    fn recv_internal(&mut self, message : interop::CrossyMessage)
    {
        let client_receive_time_us = WasmInstant::now().saturating_duration_since(self.client_start).as_micros() as u32;
        match message {
            interop::CrossyMessage::TimeResponsePacket(time_info) => {
                self.queued_time_info = Some(interop::TimeRequestEnd {
                    client_receive_time_us,
                    client_send_time_us : time_info.client_send_time_us,
                    server_receive_time_us : time_info.server_receive_time_us,
                    server_send_time_us : time_info.server_send_time_us,
                });

                //log!("Got time response, {:#?}", self.queued_time_info);
            },
            interop::CrossyMessage::ServerTick(server_tick) => {
                panic!("Removing original ServerTicks");
                //self.queued_server_messages.push_front(server_tick);
            }
            interop::CrossyMessage::LindenServerTick(linden_server_tick) => {
                self.queued_server_linden_messages.push_front(linden_server_tick);
            }
            _ => {},
        }
    }

    pub fn get_client_message(&self) -> Vec<u8>
    {
        let message = self.get_client_message_internal();
        flexbuffers::to_vec(message).unwrap()
    }

    fn get_client_message_internal(&self) -> interop::CrossyMessage
    {
        //let input = self.local_player_info.as_ref().map(|x| x.input).unwrap_or(Input::None);
        //let mut input = self.timeline.top_state().
        let input = self.local_player_info.as_ref().map(|x| self.timeline.top_state().player_inputs.get(x.player_id)).unwrap_or(Input::None);
        let message = interop::CrossyMessage::ClientTick(interop::ClientTick {
            time_us: self.last_tick,
            frame_id: self.timeline.top_state().frame_id,
            input: input,
            lobby_ready : self.ready_state,
        });

        if (input != Input::None) {
            //log!("{:?}", self.timeline.states.iter().map(|x| (x.frame_id, x.time_us)).collect::<Vec<_>>());
            //log!("{:?}", message);
        }

        message
    }

    pub fn should_get_time_request(&self) -> bool {
        let frame_id = self.timeline.top_state().frame_id;
        frame_id % TIME_REQUEST_INTERVAL == 0
    }

    pub fn get_time_request(&self) -> Vec<u8>
    {
        let message = self.get_time_request_internal();
        flexbuffers::to_vec(message).unwrap()

    }

    fn get_time_request_internal(&self) -> interop::CrossyMessage
    {
        let client_send_time_us = WasmInstant::now().saturating_duration_since(self.client_start).as_micros() as u32;
        interop::CrossyMessage::TimeRequestPacket(interop::TimeRequestPacket {
            client_send_time_us,
        })
    }

    pub fn get_players_json(&self) -> String
    {
        let time_us = self.timeline.top_state().time_us;
        let players : Vec<_> = self.timeline.top_state().get_valid_player_states()
            .iter()
            .map(|x| x.to_public(self.get_round_id(), time_us, &self.timeline.map))
            .collect();

        if (players.len() == 0)
        {
            log!("get_players_json() empty {:#?}", self.timeline.top_state());
        }

        serde_json::to_string(&players).unwrap()
    }

    // Return -1 if no local player
    pub fn get_local_player_id(&self) -> i32 {
        self.local_player_info.as_ref().map(|x| x.player_id.0 as i32).unwrap_or(-1)
    }

    pub fn get_rule_state_json(&self) -> String {
        match self.get_latest_server_rule_state() {
            Some(x) => {
                serde_json::to_string(x).unwrap()
            }
            _ => {
                "".to_owned()
            }
        }
    }

    fn get_latest_server_rule_state(&self) -> Option<&crossy_ruleset::CrossyRulesetFST> {
        /*
        let us = self.last_server_tick? + 1;
        let state_before = self.timeline.get_state_before_eq_us(us)?;
        Some(state_before.get_rule_state())
        */
        self.trusted_rule_state.as_ref()
    }

    pub fn get_rows_json(&mut self) -> String {
        serde_json::to_string(&self.get_rows()).unwrap()
    }

    fn get_rows(&mut self) -> Vec<(i32, map::Row)> {
        let mut vec = Vec::with_capacity(32);
        let screen_y = self.trusted_rule_state.as_ref().map(|x| x.get_screen_y()).unwrap_or(0);
        let range_min = screen_y;
        let range_max = (screen_y + 160/8 + 6).min(160/8);
        for i in range_min..range_max {
            let y = i;
            vec.push((y as i32, self.timeline.map.get_row(self.get_round_id(), y).clone()));
        }
        vec
    }

    pub fn get_cars_json(&self) -> String {
        let cars = self.timeline.map.get_cars(self.get_round_id(), self.timeline.top_state().time_us);
        serde_json::to_string(&cars).unwrap()
    }

    pub fn get_lillipads_json(&self) -> String {
        let lillipads = self.timeline.map.get_lillipads(self.get_round_id(), self.timeline.top_state().time_us, self.get_river_spawn_times());
        serde_json::to_string(&lillipads).unwrap()
    }

    pub fn player_alive_state_json(&self, player_id : u32) -> String {
        serde_json::to_string(&self.player_alive_state(player_id)).unwrap()
    }

    fn player_alive_state(&self, player_id : u32) -> AliveState
    {
        // We have to be careful here.
        // We dont want to tell the client a player is dead if they could possibly "come back alive".
        // For remote players we want to wait for confirmation from the server.
        // For local player we can probably make this decision earlier. (Weird edge case where player pushing you in gets interrupted before they can?)

        self.get_latest_server_rule_state().map(|x| {
            x.get_player_alive(PlayerId(player_id as u8))
        }).unwrap_or(AliveState::Unknown)
    }

    pub fn is_river(&self, y : f64) -> bool {
        match self.timeline.map.get_row(self.get_round_id(), y.round() as i32).row_type
        {
            map::RowType::River(_) => true,
            _ => false,
        }
    }

    pub fn is_path(&self, y : f64) -> bool {
        match self.timeline.map.get_row(self.get_round_id(), y.round() as i32).row_type
        {
            map::RowType::Path(_) => true,
            map::RowType::Stands() => true,
            map::RowType::StartingBarrier() => true,
            _ => false,
        }
    }

    pub fn set_ai(&mut self, ai_config : &str) {
        if (self.local_player_info.is_none())
        {
            log!("No local player to set ai on");
            return;
        }

        let local_player_id = self.local_player_info.as_ref().unwrap().player_id;

        let lower = ai_config.to_lowercase();
        match lower.as_str() {
            "none" => {
                log!("Setting ai agent to none");
                self.ai_agent = None;
            },
            "go_up" => {
                log!("Setting ai agent to 'go_up'");
                self.ai_agent = Some(RefCell::new(Box::new(ai::go_up::GoUpAI::new(local_player_id))));
            },
            _ => {
                log!("Unknown ai agent {}", ai_config);
            }
        }
    }

    fn get_ai_drawstate(&self) -> Option<ai::AIDrawState> {
        if let Some(x) = self.local_player_info.as_ref() {
            if (self.player_alive_state(x.player_id.0 as u32) != AliveState::Alive) {
                return None;
            }
        }

        self.ai_agent.as_ref().map(|x| x.borrow().get_drawstate().clone())
    }

    pub fn get_ai_drawstate_json(&self) -> String {
        match self.get_ai_drawstate() {
            Some(x) => {
                serde_json::to_string(&x).unwrap()
            }
            _ => {
                "".to_owned()
            }
        }
    }

    fn get_lilly_drawstate(&self) -> Option<Vec<LillyOverlay>> {
        self.local_player_info.as_ref().and_then(|x| {
            if (self.player_alive_state(x.player_id.0 as u32) != AliveState::Alive) {
                None
            }
            else {
                let top_state = self.timeline.top_state();
                top_state.get_player(x.player_id).and_then(|player| {
                    match &player.move_state {
                        player::MoveState::Stationary => {
                            let precise_coords = match &player.pos {
                                Pos::Coord(coord_pos) => {
                                    coord_pos.to_precise()
                                },
                                Pos::Lillipad(lilly_id) => {
                                    let x = self.timeline.map.get_lillipad_screen_x(top_state.time_us, &lilly_id);
                                    PreciseCoords {
                                        x,
                                        y : lilly_id.y,
                                    }
                                },
                            };

                            let lilly_moves = get_lilly_moves(&precise_coords, self.get_river_spawn_times(), top_state.get_round_id(), top_state.time_us, &self.timeline.map);
                            Some(lilly_moves)

                        }
                        _ => {
                            None
                        }
                    }
                })
            }
        })
    }

    pub fn get_lilly_drawstate_json(&self) -> String {
        match self.get_lilly_drawstate() {
            Some(x) => {
                serde_json::to_string(&x).unwrap()
            }
            _ => {
                "".to_owned()
            }
        }
    }
}

#[derive(serde::Serialize, Debug, Clone)]
struct LillyOverlay {
    precise_coords : PreciseCoords,
    input : Input,
}

fn get_lilly_moves(initial_pos : &PreciseCoords, spawn_times : &RiverSpawnTimes, round_id : u8, time_us : u32, map : &map::Map) -> Vec<LillyOverlay>
{
    let mut moves = vec![];

    for input in &ALL_INPUTS {
        let applied = initial_pos.apply_input(*input);
        if let Some(lilly) = map.lillipad_at_pos(round_id, spawn_times, time_us, applied) {
            let screen_x = map.get_lillipad_screen_x(time_us, &lilly);
            moves.push(LillyOverlay {
                precise_coords: PreciseCoords {
                    x : screen_x,
                    y : applied.y,
                },
                input: *input,
            });
        }
    }

    moves
}

fn try_deserialize_message(buffer : &[u8]) -> Option<interop::CrossyMessage>
{
    let reader = flexbuffers::Reader::get_root(buffer).map_err(|e| log!("{:?}", e)).ok()?;
    interop::CrossyMessage::deserialize(reader).map_err(|e| log!("{:?}", e)).ok()
}

fn dan_lerp(x0 : f32, x : f32, k : f32) -> f32 {
    (x0 * (k-1.0) + x) / k
}

fn dan_lerp_directional(x0 : f32, x : f32, k_up : f32, k_down : f32) -> f32 {
    let k = if (x > x0) {
        k_up
    }
    else {
        k_down
    };

    dan_lerp(x0, x, k)
}

fn dan_lerp_snap_thresh(x0 : f32, x : f32, k : f32, snap_thresh : f32) -> f32 {
    if (x0 - x).abs() > snap_thresh {
        x
    }
    else
    {
        dan_lerp(x0, x, k)
    }
}