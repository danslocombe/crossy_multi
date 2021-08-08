import { dan_lerp } from "./utils";
import { create_whiteout } from "./visual_effects";

const dialogue_sprites = {}

dialogue_sprites.frog = new Image(60, 46);
dialogue_sprites.frog.src = "sprites/spr_frog_dialogue.png";

//dialogue_sprites.mouse = new Image(90, 72);
dialogue_sprites.mouse = new Image(60, 64);
dialogue_sprites.mouse.src = "sprites/spr_mouse_dialogue_cute.png";

dialogue_sprites.bird = new Image(60, 64);
dialogue_sprites.bird.src = "sprites/spr_bird_dialogue_cute.png";

dialogue_sprites.snake = new Image(90, 72);
dialogue_sprites.snake.src = "sprites/spr_snake_dialogue.png";

let snd_join = new Audio('/sounds/snd_join.wav');
snd_join.volume = 0.2;

function ease_in_quad(x) {
    return 1 - (1 - x) * (1 - x);
}

function create_dialogue(sprite_name, duration = undefined) {
    console.log("Creating dialogue for " + sprite_name);
    let sprite = dialogue_sprites[sprite_name];
    if (!sprite) {
        // TODO default sprite
        sprite = dialogue_sprites.frog;
    }
    const fade_in_time = 16;
    const fade_out_time = 24;
    const target_letterbox = 30;
    const target_face_scale = 1.5;
    const face_x_off_max = 140;

    if (!duration) {
        duration = 180;
    }

    return {
        sprite : sprite,
        is_alive : true,
        t : 0,
        letterbox : 0,
        face_scale : 0,
        scale_factor : 0,
        face_x_off : 0,
        frame_id : 0,
            
        t_end : duration,

        tick : function() {
            this.t += 1;

            if (this.t < fade_in_time) {
                // Easing in
                this.scale_factor = ease_in_quad(this.t / fade_in_time);
                this.face_scale = this.scale_factor * target_face_scale;
            }
            else if (this.t_end) {
                if (this.t < this.t_end - fade_out_time) {
                    // Steady state
                    this.scale_factor = 1;
                }
                else if (this.t < this.t_end) {
                    // Easing out
                    this.scale_factor = ease_in_quad((this.t_end - this.t)/ fade_out_time);
                    this.face_x_off = (1 - this.scale_factor) * face_x_off_max;
                    this.frame_id = 1;
                }
                else {
                    // Destroy
                    this.is_alive = false;
                }
            }

            this.letterbox = this.scale_factor * target_letterbox;
        },

        trigger_close : function() {
            if (this.t_end) {
                // Already has an ending time, set to the min of the two
                this.t_end = Math.min(this.t_end, this.t + fade_out_time);
            }
            else {
                // Start destruction sequence
                this.t_end = this.t + fade_out_time;
            }
        },

        alive : function() {
            return this.is_alive;
        },

        draw : function(froggy_draw_ctx) {
            let ctx = froggy_draw_ctx.ctx;

            ctx.fillStyle = "#000000";
            ctx.fillRect(0, 0, 160, this.letterbox);
            ctx.fillRect(0, 160-this.letterbox, 160, this.letterbox);


            const x = 130 + this.face_x_off;
            const y = 35;
            const w = this.sprite.width;
            const h = this.sprite.height;
            const interval = 50;
            const spin_interval = 115;
            const w_draw = w * this.face_scale * (1 + 0.12 * Math.sin(this.t / interval));
            const h_draw = h * this.face_scale * (1 + 0.12 * Math.sin(this.t / interval));

            ctx.save();
            ctx.translate(x, y);
            ctx.rotate(0.3 * Math.sin(this.t / spin_interval));

            ctx.drawImage(
                this.sprite,
                w * this.frame_id,
                0,
                w,
                h,
                -w_draw / 2,
                -h_draw / 2,
                w_draw,
                h_draw);

            ctx.restore();
        }
    };
}

export function create_dialogue_controller() {
    return {
        dialogue_instance : undefined,

        lobby_joined_players : {},
        lobby_join_queue : [],
        lobby_first_tick : true,

        round_cooldown_first_tick : true,

        tick : function(rule_state, players, simple_entities) {
            if (rule_state && rule_state.Lobby) {
                this.tick_lobby(players, simple_entities);
            }
            else {
                this.tick_game(players, rule_state, simple_entities);
            }

            if (this.dialogue_instance)
            {
                this.dialogue_instance.tick();
                if (!this.dialogue_instance.alive())
                {
                    this.dialogue_instance = undefined;
                }
            }
        },

        tick_game : function(players, rule_state, simple_entities) {
            if (rule_state && rule_state.RoundCooldown) {
                let alive_player = false;
                let alive_player_id = 0;

                // Up to one alive player
                for (let i in rule_state.RoundCooldown.round_state.alive_players.inner) {
                    if (rule_state.RoundCooldown.round_state.alive_players.inner[i]) {
                        alive_player = true;
                        alive_player_id = i;
                    }
                }

                if (this.round_cooldown_first_tick) {
                    this.round_cooldown_first_tick = false;

                    if (alive_player) 
                    {
                        const whiteout = create_whiteout()
                        simple_entities.push(whiteout);
                        const sprite_name = players[alive_player_id].sprite_name;
                        this.dialogue_instance = create_dialogue(sprite_name);
                    }
                }
                else {
                    if (this.dialogue_instance && (!alive_player || rule_state.RoundCooldown.remaining_us < 20000)) {
                        this.dialogue_instance.trigger_close();
                    }
                }
            }
            else {
                this.round_cooldown_first_tick = true;
            }
        },

        tick_lobby : function(players, simple_entities) {
            if (players) {
                for (let player of players) {
                    if (player) {
                        const player_id = player.source.player_id;
                        if (!this.lobby_joined_players[player_id]) {
                            this.lobby_joined_players[player_id] = {};

                            if (!this.lobby_first_tick) {
                                this.lobby_join_queue.push(player.sprite_name);
                                const whiteout = create_whiteout()
                                simple_entities.push(whiteout);
                                snd_join.play();
                            }
                        }
                    }
                }
            }

            if (this.lobby_join_queue.length > 0) {
                if (!this.dialogue_instance) {
                    const sprite = this.lobby_join_queue.shift();
                    this.dialogue_instance = create_dialogue(sprite, 90);
                }
            }

            this.lobby_first_tick = false;
        },

        alive : () => true,

        draw : function(froggy_draw_ctx) {
            if (this.dialogue_instance) {
                this.dialogue_instance.draw(froggy_draw_ctx);
            }
        }
    }
}