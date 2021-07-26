import { create_player_remote, create_player_local } from "./player_def";
//import "/components/player_def";

const SCALE = 8;

const spr_car_width = 24;
const spr_car_height = 16;
let spr_car = new Image(spr_car_width, spr_car_height);
spr_car.src = '/sprites/spr_car_flipped.png';

let spr_car_flipped = new Image(spr_car_width, spr_car_height);
spr_car_flipped.src = '/sprites/spr_car.png';

let players = {};

export function create_game_view(ctx, client, ws, key_event_source) {
    let view = {
        client : client,
        ws : ws,
        ctx : ctx,
        key_event_source : key_event_source,
        current_input : "None",
        simple_entities : [],

        tick : function()
        {
            this.ctx.fillStyle = "#BAEAAA";
            this.ctx.fillRect(0, 0, 256, 256);

            if (this.client)
            {
                this.client.buffer_input_json('"' + this.current_input + '"');
                this.current_input = "None";

                this.client.tick();

                // Check if ws in ready state
                // https://developer.mozilla.org/en-US/docs/Web/API/WebSocket/readyState
                const ws_ready = this.ws.readyState == 1

                if (ws_ready)
                {
                    const client_tick = this.client.get_client_message();
                    this.ws.send(client_tick);
                }

                const rows = JSON.parse(this.client.get_rows_json());
                for (const row of rows) {
                    let y = row[0];

                    let col0, col1;

                    if (row[1].row_type.River) {
                        //col0 = "#ADD8E6";
                        col0 = "#6c6ce2";
                        col1 = "#5b5be7";
                    }
                    else if (row[1].row_type.Road) {
                        col0 = '#59595d';
                        col1 = '#646469';
                    }
                    else {
                        col0 = "#c4e6b5";
                        col1 = "#d1bfdb";
                    }

                    for (let i = 0; i < 160 / 8; i++) {
                        let x = i * 8;

                        if ((i + row[1].row_id) % 2 == 0) {
                            this.ctx.fillStyle = col0
                        }
                        else {
                            this.ctx.fillStyle = col1
                        }

                        this.ctx.fillRect(x, SCALE*y, x + 8, SCALE);
                    }
                }

                let simple_entities_new = [];//new Array(simple_entities.length);
                for (let entity of this.simple_entities) {
                    entity.tick(); 
                    entity.draw(this.ctx);
                    if (entity.alive()) {
                        simple_entities_new.push(entity);
                    }
                }

                this.simple_entities = simple_entities_new;


                const cars = JSON.parse(this.client.get_cars_json());
                //console.log(cars);
                for (const car of cars) {
                    const x = car[0] * SCALE - spr_car_width / 2;
                    const y = car[1] * SCALE - spr_car_height / 2;
                    const flipped = car[2];
                    const frame_id = 0;
                    let spr = spr_car;
                    if (flipped) {
                        spr = spr_car_flipped;
                    }
                    ctx.drawImage(spr, spr_car_width*frame_id, 0, spr_car_width, spr_car_height, x, y, spr_car_width, spr_car_height);
                }

                const players_json = this.client.get_players_json();
                const current_player_states = JSON.parse(players_json);

                // TODO fixme
                let local_player_id = this.client.get_local_player_id();
                if (local_player_id >= 0) {
                    for (const current_player_state of current_player_states) {
                        if (!players[current_player_state.id]) {
                            console.log("creating player");
                            if (current_player_state.id === local_player_id) {
                                console.log("creating local player");
                                // Create local player
                                players[current_player_state.id] = create_player_local(this.client, this.key_event_source);
                            }
                            else {
                                // Create remote player
                                players[current_player_state.id] = create_player_remote(this.client, current_player_state.id);
                            }
                        }

                        let player = players[current_player_state.id];
                        player.tick(current_player_state, this.simple_entities);
                        player.draw(this.ctx);
                    }
                }

                const rule_state = this.client.get_rule_state_json()
                document.getElementById("state").innerHTML = rule_state;
            }
        }

    }

    let listener = key_event_source.add_listener();
    listener.on_keydown = function(input) {
        view.current_input = input;
    }

    return view;
}