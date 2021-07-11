use wasm_bindgen::prelude::*;
use web_sys::console;

use crossy_multi_core::{client, game};

use std::time::{Instant, Duration};

const DESIRED_TICK_TIME : Duration = Duration::from_millis(15);


// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don't want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;


// This is like the `main` function, except for JavaScript.
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    // This provides better error messages in debug mode.
    // It's disabled in release mode so it doesn't bloat up the file size.
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();


    // Your code goes here!
    console::log_1(&JsValue::from_str("Hello world!"));

    let mut client = client::Client::try_create(8089).expect("Could not create client");
    let mut tick = 0;
    let mut cur_pos = game::Pos::Coord(game::CoordPos{x: 0, y:0});
    let mut up = true;
    loop {
        let tick_start = Instant::now();

        let input = if tick % 50 == 25 { 
            up = !up;
            if up {
                game::Input::Up
            }
            else {
                game::Input::Down
            }
        }
        else {
            game::Input::None
        };

        client.tick(input);

        {
            let top_state = client.timeline.top_state();
            let pos = top_state.get_player(client.local_player_id).unwrap().pos;
            if cur_pos != pos
            {
                cur_pos = pos;
                console::log_1(&JsValue::from_str(&format!("T = {}", top_state.time_us)));
                console::log_1(&JsValue::from_str(&format!("Pos = {:?}", &cur_pos)));
            }
        }

        let now = Instant::now();
        let elapsed_time = now.saturating_duration_since(tick_start);

        if let Some(sleep_time) = DESIRED_TICK_TIME.checked_sub(elapsed_time)
        {
            std::thread::sleep(sleep_time);
        }

        tick += 1
    }
}