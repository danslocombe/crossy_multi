#![allow(unused_parens)]
#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;

use crossy_multi_core::*;
use crossy_multi_core::crossy_ruleset::GameConfig;
use std::sync::Arc;

use warp::Filter;
use warp::reply::{Reply, Response, self};
use warp::ws::{self, Message, WebSocket};
use warp::reject::{self, Rejection};

use serde::{Serialize, Deserialize};

use tokio::sync::Mutex;
use futures::{SinkExt, StreamExt};

mod crossy_server;
mod gameid_generator;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct GameId(String);

#[derive(Clone)]
struct GameDbInner {
    id: GameId,
    game : Arc<crossy_server::Server>,
}

#[derive(Clone)]
struct GameDb {
    games : Arc<Mutex<Vec<GameDbInner>>>,
    gameid_generator : Arc<Mutex<gameid_generator::GameIdGenerator>>,
}

impl GameDb {
    fn new() -> Self {
        GameDb {
            games: Arc::new(Mutex::new(Vec::new())),
            gameid_generator: Arc::new(Mutex::new(gameid_generator::GameIdGenerator::new())),
        }
    }

    async fn new_game(&self, config : GameConfig) -> GameId {
        let mut games = self.games.lock().await;

        let id = {
            let mut idgen_lock = self.gameid_generator.lock().await;
            idgen_lock.next()
        };

        let game = Arc::new(crossy_server::Server::new(config, &id));

        games.push(GameDbInner {
            id: id.clone(),
            game : game.clone(),
        });

        tokio::task::spawn(async move {
            game.run().await;
        });

        id
    }

    async fn cleanup(&self)
    {
        let mut games_inner = self.games.lock().await;
        let mut games_swap = Vec::with_capacity(games_inner.len());

        for game in &*games_inner
        {
            let game_inner = game.game.inner.lock().await;
            if (!game_inner.ended)
            {
                games_swap.push(game.clone());
            }
            else {
                println!("Dropping {:?}", game.id);
            }
        }

        *games_inner = games_swap;
    }

    async fn get(&self, game_id : GameId) -> Result<GameDbInner, Rejection> {
        let games = self.games.lock().await;
        let m_game = games.iter().filter(|x| x.id == game_id).next();
        match m_game {
            Some(x) => {
                Ok(x.clone())
            },
            None => {
                Err(reject::not_found())
            },
        }
    }
}

const SERVE_DIR_DEV : &'static str = "C:\\Users\\Dan\\crossy_multi\\web-client\\dist";

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    let games = GameDb::new();

    let serve_dir = if let Some(arg) = std::env::args().nth(1) {
        String::from(arg)
    }
    else {
        String::from(SERVE_DIR_DEV)
    };

    println!("Serving from {}", &serve_dir);

    crossy_multi_core::set_debug_logger(Box::new(crossy_multi_core::StdoutLogger()));

    let games0 = games.clone();
    tokio::task::spawn(async move {
        const DESIRED_CLEANUP_TIME: std::time::Duration = std::time::Duration::from_millis(5000);
        loop {
            games0.cleanup().await;
            tokio::time::sleep(DESIRED_CLEANUP_TIME).await;
        }
    });

    let site = warp::fs::dir(serve_dir).with(warp::compression::gzip()).boxed();

    // GET /new
    let get_new = warp::path!("new")
        .and(warp::get())
        .and(warp::query::<NewGameOptions>())
        .and(with_db(games.clone()))
        .and(warp::any())
        .and_then(new_game_handler).boxed();

    // GET /join?game_id=1&name=dan
    let get_join = warp::path!("join")
        .and(warp::get())
        .and(warp::query::<JoinOptions>())
        .and(with_db(games.clone()))
        .and_then(join_handler).boxed();

    // GET /play?game_id=1&socket_id=1
    let get_play = warp::path!("play")
        .and(warp::get())
        .and(warp::query::<PlayOptions>())
        .and(with_db(games.clone()))
        .and_then(play_handler).boxed();
     
    // WS /ws?game_id=1&socket_id=1
    let websocket = warp::path!("ws")
        .and(warp::ws())
        .and(warp::query::<WebSocketJoinOptions>())
        .and(with_db(games.clone()))
        .and_then(ws_handler).boxed();
   
    let routes = get_new
        .or(get_join)
        .or(get_play)
        .or(site)
        .or(websocket)
        .boxed();

    let serve_from = ([0, 0, 0, 0], 8080);
    println!("Serving from {:?}", serve_from);

    warp::serve(routes)
        .run(serve_from)
        .await;
}

fn with_db(db: GameDb) -> impl Filter<Extract = (GameDb,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || db.clone())
}

#[derive(Debug, Clone, Deserialize)]
struct NewGameOptions {
    debug_bypass_lobby : Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NewGameResponse
{
    pub game_id : GameId,
}

async fn new_game_handler(options : NewGameOptions, db: GameDb) -> Result<Response, std::convert::Infallible>  {
    let mut config = GameConfig::default();

    if let Some(true) = options.debug_bypass_lobby {
        config.bypass_lobby = true;
        config.minimum_players = 1;
    }

    let game_id = db.new_game(config).await;
    let new_game_response = NewGameResponse { game_id };
    let response = warp::reply::json(&new_game_response).into_response();
    println!("/new {:?}", &response);
    Ok(response)
}

#[derive(Debug, Clone, Deserialize)]
struct JoinOptions {
    pub game_id : GameId, 
    pub name : String, 
}

#[derive(Debug, Clone, Serialize)]
struct JoinResponse {
    pub socket_id : crossy_server::SocketId,
    pub server_description : interop::ServerDescription,
    pub server_time_us : u32,
    pub server_frame_id : u32,
}

async fn join_handler(options : JoinOptions, db: GameDb) -> Result<Response, Rejection>  {
    println!("Join with options {options:?}");
    let dbinner = db.get(options.game_id).await?;
    let server_description = dbinner.game.get_server_description().await;
    //let last_frame_time_us = dbinner.game.get_last_frame_time_us().await;
    let socket_id = dbinner.game.join().await;
    let server_time_us = dbinner.game.time_since().await;
    let server_frame_id = dbinner.game.frame_id().await;
    let response = JoinResponse {
        socket_id,
        server_description,
        server_time_us : server_time_us.as_micros() as u32,
        server_frame_id,
    };

    Ok(reply::json(&response).into_response())
}

#[derive(Debug, Clone, Deserialize)]
struct PlayOptions {
    pub game_id : GameId, 
    pub socket_id : crossy_server::SocketId,
}

async fn play_handler(options: PlayOptions, db: GameDb) -> Result<Response, Rejection>  {
    println!("Play with options {options:?}");
    let dbinner = db.get(options.game_id).await?;
    let hello = interop::ClientHello::default();
    let init_server_response = dbinner.game.play(&hello, options.socket_id).await;
    Ok(reply::json(&init_server_response).into_response())
}

#[derive(Debug, Clone, Deserialize)]
struct WebSocketJoinOptions {
    pub game_id : GameId, 
    pub socket_id : crossy_server::SocketId, 
}

#[derive(Debug, Clone, Deserialize)]
struct StartTimeUtcOptions {
    pub game_id : GameId, 
}

async fn start_time_utc_handler(options: StartTimeUtcOptions, db: GameDb) -> Result<Response, Rejection>  {
    println!("Start time with options {options:?}");
    let dbinner = db.get(options.game_id).await?;
    let time = dbinner.game.get_start_time_utc().await;
    Ok(reply::json(&time).into_response())
}

async fn ws_handler(ws : ws::Ws, options: WebSocketJoinOptions, db : GameDb) -> Result<Response, Rejection> {
    println!("WS Handler");

    let game = db.get(options.game_id).await?;

    let socket_id = options.socket_id;
    Ok(ws.on_upgrade(move |socket| {
        websocket_main(socket, game, socket_id)
    }).into_response())
}

async fn websocket_main(ws: WebSocket, db : GameDbInner, socket_id : crossy_server::SocketId) {
    println!("Websocket connected");

    let mut tick_listener = db.game.get_listener();
    let game_start = db.game.get_start_time().await;
    let (mut ws_tx, mut ws_rx) = ws.split();


    tokio::task::spawn(async move {
        loop {
            match tick_listener.recv().await {
                Ok(crossy_multi_core::interop::CrossyMessage::GoodBye()) => {
                    println!("Game ended cleaning up WS listener");
                    break;
                },
                Ok(mut to_send) => {

                    // Special case handling for time request responses
                    if let interop::CrossyMessage::TimeRequestIntermediate(time_request_state) = &to_send {
                        //println!("Time request packet {:#?}", time_request_state);
                        if (time_request_state.socket_id != socket_id.0)
                        {
                            // Not for us
                            // this kinda sucks, todo improve
                            continue;
                        }

                        let server_send_time_us = std::time::Instant::now().saturating_duration_since(game_start).as_micros() as u32;
                        to_send = interop::CrossyMessage::TimeResponsePacket(interop::TimeResponsePacket{
                            client_send_time_us : time_request_state.client_send_time_us,
                            server_receive_time_us : time_request_state.server_receive_time_us,
                            server_send_time_us,
                        });
                    }

                    let serialized = flexbuffers::to_vec(&to_send).unwrap();
                    match ws_tx.send(Message::binary(serialized)).await
                    {
                        Ok(_) => {},
                        Err(e) => {println!("Websocket send error {e}"); break;}
                    }
                },
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    println!("[{:?}] Underlying game closed", socket_id);
                    break;
                },
                Err(tokio::sync::broadcast::error::RecvError::Lagged(x)) => {
                    println!("[{:?}] Client lagged by {}", socket_id, x);
                },
            }
        }
    });

    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(msg) =>
            {
                if let Some(message) = parse_client_message(&msg)
                {
                    db.game.queue_message(message, socket_id).await;
                }
            }
            Err(e) => {
                println!("Client receive err {e}");
                break;
            }
        }
    }

    println!("Client disconnected");
    db.game.queue_message(interop::CrossyMessage::ClientDrop{}, socket_id).await;
}

fn parse_client_message(ws_message : &warp::ws::Message) -> Option<interop::CrossyMessage>
{
    let bytes = ws_message.as_bytes();
    let r = flexbuffers::Reader::get_root(bytes).map_err(|e| println!("{e}")).ok()?;
    interop::CrossyMessage::deserialize(r).map_err(|e| println!("{e}")).ok()
}