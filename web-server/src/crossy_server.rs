use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crossy_multi_core::game::{self, GameState};
use crossy_multi_core::interop::*;
use crossy_multi_core::map::Map;
use crossy_multi_core::player_id_map::PlayerIdMap;
use crossy_multi_core::timeline::{RemoteInput, RemoteTickState, Timeline};

const SERVER_VERSION: u8 = 1;
const DESIRED_TICK_TIME: Duration = Duration::from_millis(14);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocketId(pub u32);

struct PlayerClient {
    id: game::PlayerId,
    last_tick_us: u32,
}

struct Client {
    player_client: Option<PlayerClient>,
    socket_id: SocketId,
}

pub struct Server {
    queued_messages: Mutex<Vec<(CrossyMessage, SocketId, Instant)>>,
    pub inner: Mutex<ServerInner>,

    outbound_tx: tokio::sync::broadcast::Sender<CrossyMessage>,
    outbound_rx: tokio::sync::broadcast::Receiver<CrossyMessage>,
}

pub struct ServerInner {
    game_id: crate::GameId,
    empty_ticks: u32,
    new_players: Vec<game::PlayerId>,
    start: Instant,
    start_utc: DateTime<Utc>,
    prev_tick: Instant,
    clients: Vec<Client>,

    last_client_sent: PlayerIdMap<RemoteTickState>,

    //timeline: Timeline,
    state: ServerState,

    next_socket_id: SocketId,
    pub ended: bool,
}

pub struct ServerState {
    game_state: GameState,
    map: Map,
}

impl Server {
    pub fn new(id: &crate::GameId) -> Self {
        let start = Instant::now();
        let start_utc = Utc::now();
        let (outbound_tx, outbound_rx) = tokio::sync::broadcast::channel(16);

        let server_state = ServerState {
            game_state: GameState::new(),
            map: Map::new(&id.0),
        };

        Server {
            queued_messages: Mutex::new(Vec::new()),
            outbound_tx,
            outbound_rx,
            inner: Mutex::new(ServerInner {
                game_id: id.clone(),
                empty_ticks: 0,
                clients: Vec::new(),
                new_players: Vec::new(),
                //timeline: Timeline::from_seed(&id.0),
                state: server_state,
                last_client_sent: Default::default(),

                prev_tick: start,
                start,
                start_utc,
                next_socket_id: SocketId(0),
                ended: false,
            }),
        }
    }

    pub async fn queue_message(&self, message: CrossyMessage, player: SocketId) {
        let now = Instant::now();
        match message {
            CrossyMessage::TimeRequestPacket(time_request) => {
                // Special case hanling for time requests to track the exact time we received the message.

                let inner_guard = self.inner.lock().await;
                let server_receive_time_us =
                    now.saturating_duration_since(inner_guard.start).as_micros() as u32;
                drop(inner_guard);

                let new_message = CrossyMessage::TimeRequestIntermediate(TimeRequestIntermediate {
                    server_receive_time_us,
                    client_send_time_us: time_request.client_send_time_us,
                    socket_id: player.0,
                });

                let mut queue_guard = self.queued_messages.lock().await;
                queue_guard.push((new_message, player, now));
            }
            _ => {
                let mut guard = self.queued_messages.lock().await;
                guard.push((message, player, now));
            }
        }
    }

    pub async fn get_server_description(&self) -> ServerDescription {
        let inner = self.inner.lock().await;
        ServerDescription {
            server_version: SERVER_VERSION,
            seed: inner.state.map.get_seed(),
        }
    }

    pub async fn join(&self) -> SocketId {
        let mut inner = self.inner.lock().await;
        println!("[{:?}] /join", inner.game_id);
        inner.add_client()
    }

    pub async fn time_since(&self) -> Duration {
        let inner = self.inner.lock().await;
        let now = Instant::now();
        now.saturating_duration_since(inner.start)
    }

    pub async fn get_start_time_utc(&self) -> String {
        println!("/start_time_utc");
        let inner = self.inner.lock().await;
        inner.start_utc.to_string()
    }

    pub async fn play(
        &self,
        hello: &ClientHello,
        socket_id: SocketId,
    ) -> Option<InitServerResponse> {
        let mut inner = self.inner.lock().await;

        println!(
            "[{:?}] /play {:?} {:?} looks ok: {}",
            inner.game_id,
            socket_id,
            &hello,
            hello.check(1)
        );

        let client_id = game::PlayerId(inner.clients.len() as u8);
        inner.new_players.push(client_id);

        // Fails if socket_id not found
        // In prod version dont crash here?
        let mut client = inner
            .get_client_mut_by_addr(socket_id)
            .expect("client tried to /play without calling /join");
        client.player_client = Some(PlayerClient {
            id: client_id,
            last_tick_us: 0,
        });

        Some(InitServerResponse {
            server_version: SERVER_VERSION,
            //player_count: inner.timeline.player_count,
            // unused I think, clean up
            player_count: 0,
            seed: inner.state.map.get_seed(),
            player_id: client_id,
        })
    }

    pub fn get_listener(&self) -> tokio::sync::broadcast::Receiver<CrossyMessage> {
        self.outbound_tx.subscribe()
    }

    pub async fn get_start_time(&self) -> Instant {
        let inner = self.inner.lock().await;
        inner.start
    }

    pub async fn get_last_frame_time_us(&self) -> u32 {
        let inner = self.inner.lock().await;
        inner.state.game_state.time_us
    }

    pub async fn run(&self) {
        // Still have client listeners
        //while self.outbound_tx.receiver_count() > 0 {
        loop {
            let tick_start = Instant::now();
            let (mut client_updates, dropped_players, ready_players) = self.receive_updates().await;

            let mut inner = self.inner.lock().await;

            // Fetch + clear list of new players
            let new_players = std::mem::take(&mut inner.new_players);

            for new_player in new_players {
                // We need to make sure this gets propagated properly
                // Weird edge case bugs
                println!(
                    "[{:?}] In run, adding a new player {:?}",
                    inner.game_id, new_player
                );
                let spawn_pos = find_spawn_pos(&inner.state.game_state);
                println!(
                    "[{:?}] Spawning new player at {:?}",
                    inner.game_id, spawn_pos
                );
                inner.state.game_state = inner.state.game_state.add_player(new_player, spawn_pos);
            }

            for dropped_player in dropped_players {
                println!("[{:?}] Dropping player {:?}", inner.game_id, dropped_player);
                inner.state.game_state = inner.state.game_state.remove_player(dropped_player);
            }

            for (ready_player, ready) in ready_players {
                inner.state.game_state =
                    inner.state.game_state.set_player_ready(ready_player, true);
            }

            // Do simulations
            let simulation_time_start = Instant::now();
            let dt_simulation = simulation_time_start.saturating_duration_since(inner.prev_tick);
            inner.prev_tick = simulation_time_start;

            let dt_simulation_us = dt_simulation.as_micros() as u32;
            //println!("Ticking dt {}", dt_simulation_us);

            let mut player_inputs = game::PlayerInputs::new();
            for (update, _) in &client_updates {
                if (update.input != game::Input::None) {
                    player_inputs.set(update.player_id, update.input);
                }
            }

            let mut new_state = inner.state.game_state.simulate(
                Some(player_inputs),
                dt_simulation_us,
                &inner.state.map,
            );

            std::mem::swap(&mut inner.state.game_state, &mut new_state);

            // Renaming
            let old_state = new_state;

            //println!("tick");

            for (update, _) in &client_updates {
                let is_still = inner
                    .state
                    .game_state
                    .get_player(update.player_id)
                    .map(|x| match x.move_state {
                        crossy_multi_core::player::MoveState::Moving(_) => false,
                        _ => true,
                    })
                    .unwrap_or(true);

                let has_update = update.input != game::Input::None;

                //if (is_still) {
                //{
                //let x = RemoteTickState {
                //time_us: inner.state.game_state.time_us,
                //states: inner.state.game_state.get_valid_player_states(),
                //};

                //inner.last_client_sent.set(update.player_id, x);
                //}

                if (has_update) {
                    println!("\n\n{:?}", update);
                    for (id, state) in inner.state.game_state.player_states.iter() {
                        println!(
                            "[{}] Prev: Remaining ms {}",
                            id.0,
                            state.move_cooldown / 1000
                        );
                    }
                    for (id, state) in inner.state.game_state.player_states.iter() {
                        match &state.move_state {
                            crossy_multi_core::player::MoveState::Moving(m) => {
                                println!(
                                    "[{}] Moving->({:?}, remaining_us: {})",
                                    id.0, m.target, m.remaining_us
                                )
                            }
                            _ => println!("[{}] {:?}", id.0, state.pos),
                        }
                    }
                }
            }

            //inner.timeline.tick(None, dt_simulation.as_micros() as u32);
            /*

            {
                // TMP Assertion
                let current_time = inner.timeline.top_state().time_us;
                for (update, _) in &mut client_updates {
                    if (update.time_us > current_time) {
                        println!("Update from the future from {:?} - ahead {}us - client time {}us server time {}us", update.player_id, update.time_us.saturating_sub(current_time), update.time_us, current_time);
                        // HACK this shitttt
                        update.time_us = current_time - 1;
                    }
                }
            }

            let nonempty_updates: Vec<_> = client_updates
                .iter()
                .filter(|(x, _)| x.input != game::Input::None)
                .cloned()
                .collect();

            for (update, receive_time) in &nonempty_updates {
                let receive_time_us = receive_time
                    .saturating_duration_since(inner.start)
                    .as_micros() as u32;
                let delta = (update.time_us as f32 - receive_time_us as f32) / 1000.;
                //let delta = (update.time_us as i32 - inner.timeline.top_state().time_us as i32) / 1000;
                println!(
                    "[{:?}] Update - {:?} at client time {}ms, receive_time {}ms, delta {}ms",
                    update.player_id,
                    update.input,
                    update.time_us / 1000,
                    receive_time_us / 1000,
                    delta.floor()
                );
            }

            if (nonempty_updates.len() > 0) {
                //let top_state_before = inner.timeline.top_state().clone();
                inner
                    .timeline
                    .propagate_inputs(nonempty_updates.into_iter().map(|(x, _)| x).collect());
                //let top_state_after = inner.timeline.top_state();

                //if (top_state_before.player_states.count_populated() != top_state_after.player_states.count_populated())
                //{
                    //println!("Different player counts");
                //}
                //else
                //{
                    //for (pid, before_state) in top_state_before.player_states.iter() {
                        //if let Some(new) = top_state_after.get_player(pid)
                        //{
                            ////if (before_state.pos != new.pos)
                            //{
                                //println!("Player {:?} {:?} -> {:?}", pid, before_state, new);
                            //}
                        //}
                    //}
                //}
            }

            for new_player in new_players {
                // We need to make sure this gets propagated properly
                // Weird edge case bugs
                println!(
                    "[{:?}] In run, adding a new player {:?}",
                    inner.game_id, new_player
                );
                let spawn_pos = find_spawn_pos(inner.timeline.top_state());
                println!(
                    "[{:?}] Spawning new player at {:?}",
                    inner.game_id, spawn_pos
                );
                inner.timeline.add_player(new_player, spawn_pos);
            }

            for dropped_player in dropped_players {
                println!("[{:?}] Dropping player {:?}", inner.game_id, dropped_player);
                inner.timeline.remove_player(dropped_player);
            }

            for (ready_player, ready) in ready_players {
                inner.timeline.set_player_ready(ready_player, ready);
            }
            */

            // Send responses
            let top_state = &inner.state.game_state;

            /*
            let mut last_client_sent = PlayerIdMap::new();
            for client in (&inner.clients)
                .iter()
                .filter_map(|x| x.player_client.as_ref())
            {
                inner
                    .timeline
                    .get_state_before_eq_us(client.last_tick_us)
                    .map(|x| {
                        last_client_sent.set(
                            client.id,
                            RemoteTickState {
                                time_us: x.time_us,
                                states: x.get_valid_player_states(),
                            },
                        );
                    });
            }
            */

            let tick = CrossyMessage::ServerTick(ServerTick {
                exact_send_server_time_us: Instant::now()
                    .saturating_duration_since(inner.start)
                    .as_micros() as u32,

                latest: RemoteTickState {
                    time_us: top_state.time_us,
                    states: top_state.get_valid_player_states(),
                },

                last_client_sent: inner.last_client_sent.clone(),

                // If an input comes in late that affects state change then this ignores it
                // Do we care?
                // Do we need some lookback period here?
                rule_state: top_state.get_rule_state().clone(),
            });

            if top_state.frame_id as usize % 300 == 0 {
                //println!("Sending tick {:?}", tick);
            }

            if (self.outbound_tx.receiver_count() <= 1) {
                inner.empty_ticks += 1;
            } else {
                inner.empty_ticks = 0;
            }

            const EMPTY_TICKS_THERSHOLD: u32 = 60 * 20;
            if (inner.empty_ticks > EMPTY_TICKS_THERSHOLD) {
                // Noone left listening, shut down
                println!("[{:?}] Shutting down game", inner.game_id);
                self.outbound_tx.send(CrossyMessage::GoodBye()).unwrap();
                inner.ended = true;
                return;
            }

            self.outbound_tx.send(tick).unwrap();

            let now = Instant::now();
            let elapsed_time = now.saturating_duration_since(tick_start);
            if let Some(sleep_time) = DESIRED_TICK_TIME.checked_sub(elapsed_time) {
                tokio::time::sleep(sleep_time).await;
            }
        }
    }

    async fn receive_updates(
        &self,
    ) -> (
        Vec<(RemoteInput, Instant)>,
        Vec<game::PlayerId>,
        Vec<(game::PlayerId, bool)>,
    ) {
        let mut queued_messages = Vec::with_capacity(8);

        let mut guard = self.queued_messages.lock().await;
        std::mem::swap(&mut queued_messages, &mut guard);
        drop(guard);

        let mut client_updates = Vec::new();
        let mut ready_players = Vec::new();

        let mut inner = self.inner.lock().await;
        let mut dropped_players = vec![];

        while let Some((message, socket_id, receive_time)) = queued_messages.pop() {
            match message {
                CrossyMessage::ClientTick(t) => match inner.get_client_mut_by_addr(socket_id) {
                    Some(client) => {
                        if let Some(player_client) = client.player_client.as_mut() {
                            let client_time = t.time_us;
                            player_client.last_tick_us = client_time;

                            ready_players.push((player_client.id, t.lobby_ready));

                            client_updates.push((
                                RemoteInput {
                                    time_us: client_time,
                                    input: t.input,
                                    player_id: player_client.id,
                                },
                                receive_time,
                            ));
                        } else {
                            println!("Received client update from client who has not called /play");
                        }
                    }
                    None => {
                        println!("Did not recognise addr {:?}", &socket_id);
                    }
                },
                CrossyMessage::ClientDrop() => {
                    if let Some(client) = inner.get_client_mut_by_addr(socket_id) {
                        if let Some(player_client) = client.player_client.as_ref() {
                            dropped_players.push(player_client.id);
                        }
                    }
                }
                CrossyMessage::TimeRequestIntermediate(time_request) => {
                    // Just forward straight over
                    self.outbound_tx
                        .send(CrossyMessage::TimeRequestIntermediate(time_request))
                        .unwrap();
                }
                _ => {}
            }
        }

        (client_updates, dropped_players, ready_players)
    }
}

impl ServerInner {
    fn add_client(&mut self) -> SocketId {
        let socket_id = self.next_socket_id;
        self.next_socket_id = SocketId(socket_id.0 + 1);
        self.clients.push(Client {
            player_client: None,
            socket_id,
        });

        socket_id
    }

    fn get_client_mut_by_addr(&mut self, id: SocketId) -> Option<&mut Client> {
        for client in &mut self.clients {
            if client.socket_id == id {
                return Some(client);
            }
        }

        None
    }
    fn get_client_by_addr(&self, id: SocketId) -> Option<&Client> {
        for client in &self.clients {
            if client.socket_id == id {
                return Some(client);
            }
        }

        None
    }
}

fn find_spawn_pos(game_state: &crossy_multi_core::game::GameState) -> crossy_multi_core::Pos {
    for x in 7..=13 {
        for y in 7..=13 {
            let spawn_pos = game::Pos::new_coord(x, y);
            if (!game_state.space_occupied_with_player(spawn_pos, None)) {
                return spawn_pos;
            }
        }
    }

    panic!("Impossible, without 36 players");
}
