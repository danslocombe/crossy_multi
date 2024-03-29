// We have some combersome names that are easier to read with underscores.
#![allow(non_camel_case_types)]

use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use crate::game::Input;
use crate::timeline::RemoteTickState;
use crate::player_id_map::PlayerIdMap;
use crate::crossy_ruleset::RulesState;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CrossyMessage {
    Hello(ClientHello),
    HelloResponse(InitServerResponse),
    ServerDecription(ServerDescription),
    ClientTick(Vec<ClientTick>),
    ClientDrop(),
    LindenServerTick(LindenServerTick),

    TimeRequestPacket(TimeRequestPacket),
    TimeRequestIntermediate(TimeRequestIntermediate),
    TimeResponsePacket(TimeResponsePacket),

    TelemetryMessagePackage(TelemetryMessagePackage),

    GoodBye(),

    EmptyMessage(),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ClientHello {
    header: [u8; 4],
    version: u8,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct InitServerResponse {
    pub server_version: u8,
    pub player_count: u8,
    pub seed: u32,
    pub player_id: crate::game::PlayerId,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ServerDescription {
    pub server_version: u8,
    pub seed: u32,
}

pub const INIT_MESSAGE: &[u8; 4] = b"helo";
pub const CURRENT_VERSION: u8 = 1;

impl Default for ClientHello {
    fn default() -> Self {
        ClientHello {
            header: *INIT_MESSAGE,
            version: CURRENT_VERSION,
        }
    }
}

impl ClientHello {
    pub fn check(&self, required_version: u8) -> bool {
        self.header == *INIT_MESSAGE && self.version >= required_version
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ClientTick {
    pub time_us: u32,
    pub frame_id: u32,
    pub input: Input,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LindenServerTick {
    pub latest : RemoteTickState,
    pub lkg_state : crate::game::GameState,
    pub delta_inputs : Vec<crate::timeline::RemoteInput>,
    pub last_client_frame_id : PlayerIdMap<u32>,
    pub rules_state : RulesState,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct TimeRequestPacket
{
    pub client_send_time_us : u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct TimeRequestIntermediate
{
    pub client_send_time_us : u32,
    pub server_receive_time_us : u32,
    // HACKY only server understands this type
    pub socket_id : u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct TimeResponsePacket
{
    pub client_send_time_us : u32,
    pub server_receive_time_us : u32,
    pub server_send_time_us : u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct TimeRequestEnd
{
    pub client_send_time_us : u32,
    pub client_receive_time_us : u32,
    pub server_receive_time_us : u32,
    pub server_send_time_us : u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct TelemetryMessagePackage
{
    pub messages : Vec<TelemetryMessage>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum TelemetryMessage
{
    ClientReceiveEvent(Telemetry_ClientReceiveEvent),
    LatencyEstimate(Telemetry_LatencyEstimate),
    PingOutcome(Telemetry_PingOutcome),
}


#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Telemetry_ClientReceiveEvent
{
    pub server_send_frame_id: u32,
    pub receive_frame_id: u32,
    //pub delta_input_server_frame_times : Vec<u32>,
    pub delta_input_server_frame_times_min : Option<u32>,
    pub delta_input_server_frame_times_max : Option<u32>,
    pub delta_input_server_frame_times_count : u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Telemetry_LatencyEstimate
{
    pub estimated_latency_us : i32,
    pub estimated_frame_delta : i32,
    pub estimated_server_current_frame_id : u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Telemetry_PingOutcome
{
    pub unlerped_estimated_latency_us : i64,
    pub unlerped_estimated_frame_delta : i64,
    pub estimated_latency_us : f32,
    pub estimated_frame_delta : f32,

    pub estimated_server_time_us : u32,
    pub estimated_server_current_frame_id : u32,

    pub current_client_time_ms : u32,
    pub current_client_date_time_ms : u32,
}