import { ease_in_quad } from "./utils"

let spr_countdown = new Image(48, 32);
spr_countdown.src = '/sprites/spr_countdown.png';

let spr_winner = new Image(113, 32);
spr_winner.src = '/sprites/spr_winner.png';

let spr_no_winner = new Image(113, 64);
spr_no_winner.src = '/sprites/spr_no_winner.png';

let snd_countdown = new Audio('/sounds/snd_countdown.wav');
snd_countdown.volume = 0.25;
let snd_countdown_go = new Audio('/sounds/snd_countdown_go.wav');
snd_countdown_go.volume = 0.25;

export function create_countdown(audio_manager) {
    return {
        enabled : false,
        time : 0,
        go_time : 0,
        audio_manager : audio_manager,

        tick : function (rules_state) {
            if (!rules_state) {
                return;
            }

            if (rules_state.fst.RoundWarmup) {
                const time = Math.ceil(rules_state.fst.RoundWarmup.remaining_us / 1000000);
                //console.log(rules_state.fst.RoundWarmup);
                if (time != this.time) {
                    this.audio_manager.play(snd_countdown);
                }
                this.time = time;
                this.enabled = true;
                this.go_time = 60;
            }
            else if (rules_state.fst.Round) {
                if (this.go_time > 0) {
                    if (this.time == 1) {
                        // First tick of "go"
                        this.audio_manager.play(snd_countdown_go);
                        this.time = 0;
                    }
                    this.go_time -= 1;
                    this.enabled = true;
                }
                else {
                    this.enabled = false;
                }
            }
            else {
                this.enabled = false;
            }
        },

        draw : function(crossy_draw_ctx) {
            if (this.enabled) {
                const frame_id = 3 - this.time;
                const x = 80 - 24;
                const y = 80 - 16;
                crossy_draw_ctx.ctx.drawImage(spr_countdown, 48*frame_id, 0, 48, 32, x, y, 48, 32);
            }
        }
    }
}

export function create_countdown_font(audio_manager, font_controller) {
    return {
        enabled : false,
        time : 0,
        go_time : 0,
        audio_manager : audio_manager,
        font_controller : font_controller,
        text: "",

        tick : function (rules_state) {
            if (!rules_state) {
                return;
            }

            if (rules_state.fst.RoundWarmup) {
                const time = Math.ceil(rules_state.fst.RoundWarmup.remaining_us / 1000000);
                //console.log(rules_state.fst.RoundWarmup);
                if (time != this.time) {
                    this.audio_manager.play(snd_countdown);
                }
                this.time = time;
                this.enabled = true;
                this.go_time = 60;
            }
            else if (rules_state.fst.Round) {
                if (this.go_time > 0) {
                    if (this.time == 1) {
                        // First tick of "go"
                        this.audio_manager.play(snd_countdown_go);
                        this.time = 0;
                    }
                    this.go_time -= 1;
                    this.enabled = true;
                }
                else {
                    this.enabled = false;
                }
            }
            else {
                this.enabled = false;
            }

            const frame_id = 3 - this.time;
            if (frame_id == 0) {
                this.text = "three";
            }
            if (frame_id == 1) {
                this.text = "two";
            }
            if (frame_id == 2) {
                this.text = "one";
            }
            if (frame_id == 3) {
                this.text = "go";
            }
        },

        draw : function(crossy_draw_ctx) {
            if (this.enabled) {

                const w = this.text.length * this.font_controller.font.width;
                const h = this.font_controller.font.height;

                this.font_controller.set_font_blob();
                this.font_controller.text(crossy_draw_ctx, this.text, 80 - w / 2, 80);
            }
        }
    }
}

export function create_winner_ui() {
    return {
        foreground_depth : 5,
        is_alive : true,
        fade_in_time : 16,
        fade_out_time : 24,
        target_scale : 1,
        scale : 0,
        scale_factor : 0,
        t : 0,
        t_end : 180,
        spr : spr_winner,
        no_winner : false,

        alive : function(x) {
            return this.is_alive;
        },

        trigger_no_winner : function() {
            this.spr = spr_no_winner;
            this.no_winner = true;
            //this.is_alive = false;
        },

        tick : function (rules_state) {
            this.t += 1;

            if (this.t < this.fade_in_time) {
                // Easing in
                this.scale_factor = ease_in_quad(this.t / this.fade_in_time);
            }
            else {
                this.scale_factor = 1;
            }

            this.scale = this.scale_factor * this.target_scale;
        },

        draw : function(crossy_draw_ctx) {

            const w = this.spr.width;
            const h = this.spr.height;
            const interval = 50;
            const spin_interval = -105;

            let w_draw = w;
            let h_draw = h;

            if (!this.no_winner) {
                w_draw = w * this.scale * (1 + 0.12 * Math.sin(this.t / interval));
                h_draw = h * this.scale * (1 + 0.12 * Math.sin(this.t / interval));
            }

            let ctx = crossy_draw_ctx.ctx;
            ctx.save();
            const x = 80;
            const y = 80;
            ctx.translate(x, y);

            if (!this.no_winner) {
                ctx.rotate(0.3 * Math.sin(this.t / spin_interval));
            }

            ctx.drawImage(
                this.spr,
                0,
                0,
                w,
                h,
                -w_draw / 2,
                -h_draw / 2,
                w_draw,
                h_draw);

            ctx.restore();
        }
    }
}

export function create_winner_ui_font(font_controller) {
    return {
        foreground_depth : 5,
        is_alive : true,
        t : 0,
        t_end : 180,
        no_winner : false,
        text : "winner",
        font_controller : font_controller,

        alive : function(x) {
            return this.is_alive;
        },

        trigger_no_winner : function() {
            this.text = "no winner";
            this.no_winner = true;
        },

        tick : function (rules_state) {
            this.t += 1;
        },

        draw : function(crossy_draw_ctx) {

            const w = this.text.length * this.font_controller.font.width;
            const h = this.font_controller.font.height;

            this.font_controller.text(crossy_draw_ctx, this.text, 80 - w / 2, 80);
        }
    }
}

export function create_game_winner_ui(font_controller, winning_player_name) {
    return {
        foreground_depth : 5,
        is_alive : true,
        t : 0,
        t_end : 180,
        no_winner : false,
        winning_player_name: winning_player_name,
        font_controller : font_controller,

        alive : function(x) {
            return this.is_alive;
        },

        tick : function (rules_state) {
            this.t += 1;
        },

        draw : function(crossy_draw_ctx) {

            this.font_controller.set_font_blob();

            const h = this.font_controller.font.height;
            this.font_controller.text(crossy_draw_ctx, "congrats", 80 - "congrats".length * this.font_controller.font.width / 2, 80 - h / 2);

            const w = this.winning_player_name.length * this.font_controller.font.width;
            this.font_controller.text(crossy_draw_ctx, this.winning_player_name, 80 - w / 2, 80 + h / 2);
        }
    }
}