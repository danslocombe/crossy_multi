use crossy_multi_core::game;

use crate::{gamepad_pressed, key_pressed};

// For now this is just if we are running in steam mode.
//static mut g_steam_input: bool = crate::STEAM;
// @HACK WHILE FIXING
pub static mut g_steam_input: bool = false;

pub fn using_steam_input() -> bool {
    unsafe { g_steam_input }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuInput {
    None,
    Up,
    Down,
    Left,
    Right,
    Select,
    ReturnToGame,
}

impl MenuInput {
    pub fn is_toggle(self) -> bool {
        match self {
            MenuInput::Left | MenuInput::Right | MenuInput::Select => true,
            _ => false
        }
    }

    pub fn read() -> (Self, Option<i32>, Option<u64>) {
        let input = Self::read_raylib_keyboard();
        if (input != Self::None) {
            return (input, None, None);
        }

        #[cfg(feature = "steam")]
        {
            if (using_steam_input()) {
                let (input, controller_id) = crate::steam::read_menu_input();
                return (input, None, if controller_id != 0 { Some(controller_id) } else { None });
            }
        }

        let (input, controller_id) = Self::read_raylib_controllers();
        (input, controller_id, None)
    }

    pub fn read_raylib_keyboard() -> Self {
        if key_pressed(raylib_sys::KeyboardKey::KEY_UP) {
            return MenuInput::Up;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_LEFT) {
            return MenuInput::Left;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_DOWN) {
            return MenuInput::Down;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_RIGHT) {
            return MenuInput::Right;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_SPACE) {
            return MenuInput::Select;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_ENTER) {
            return MenuInput::Select;
        }

        if key_pressed(raylib_sys::KeyboardKey::KEY_W) {
            return MenuInput::Up;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_A) {
            return MenuInput::Left;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_S) {
            return MenuInput::Down;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_D) {
            return MenuInput::Right;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_Z) {
            return MenuInput::Select;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_X) {
            return MenuInput::Select;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_F) {
            return MenuInput::Select;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_G) {
            return MenuInput::Select;
        }

        Self::None
    }

    pub fn read_raylib_controllers() -> (Self, Option<i32>) {
        for i in 0..4 {
            let gamepad_id = i as i32;
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_UP) {
                return (MenuInput::Up, Some(gamepad_id));
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_LEFT) {
                return (MenuInput::Left, Some(gamepad_id));
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_DOWN) {
                return (MenuInput::Down, Some(gamepad_id));
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_RIGHT) {
                return (MenuInput::Right, Some(gamepad_id));
            }

            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_LEFT) {
                return (MenuInput::Select, Some(gamepad_id));
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_RIGHT) {
                return (MenuInput::Select, Some(gamepad_id));
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_UP) {
                return (MenuInput::Select, Some(gamepad_id));
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_DOWN) {
                return (MenuInput::Select, Some(gamepad_id));
            }
        }

        (Self::None, None)
    }
}

pub fn arrow_game_input() -> game::Input {
    if (!crate::console::eating_input()) {
        if (key_pressed(raylib_sys::KeyboardKey::KEY_LEFT)) {
            return game::Input::Left;
        }
        if (key_pressed(raylib_sys::KeyboardKey::KEY_RIGHT)) {
            return game::Input::Right;
        }
        if (key_pressed(raylib_sys::KeyboardKey::KEY_UP)) {
            return game::Input::Up;
        }
        if (key_pressed(raylib_sys::KeyboardKey::KEY_DOWN)) {
            return game::Input::Down;
        }
    }

    game::Input::None
}

pub fn wasd_game_input() -> game::Input {
    if (!crate::console::eating_input()) {
        if (key_pressed(raylib_sys::KeyboardKey::KEY_A)) {
            return game::Input::Left;
        }
        if (key_pressed(raylib_sys::KeyboardKey::KEY_D)) {
            return game::Input::Right;
        }
        if (key_pressed(raylib_sys::KeyboardKey::KEY_W)) {
            return game::Input::Up;
        }
        if (key_pressed(raylib_sys::KeyboardKey::KEY_S)) {
            return game::Input::Down;
        }
    }

    game::Input::None
}


pub fn game_input_controller_raylib(gamepad_id: i32) -> game::Input {
    if (unsafe { raylib_sys::IsGamepadAvailable(gamepad_id) })
    {
        if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_LEFT) {
            return game::Input::Left;
        }
        if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_RIGHT) {
            return game::Input::Right;
        }
        if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_UP) {
            return game::Input::Up;
        }
        if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_DOWN) {
            return game::Input::Down;
        }
    }

    game::Input::None
    /*
    if (unsafe { raylib_sys::IsGamepadAvailable(gamepad_id) })
    {
        {
            let mut input = Input::None;
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_LEFT) {
                input = Input::Left;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_RIGHT) {
                input = Input::Right;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_UP) {
                input = Input::Up;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_DOWN) {
                input = Input::Down;
            }
            Self::process_input(&mut self.controller_a_players[gamepad_id as usize], input, &mut player_inputs, timeline, players_local, outfit_switchers, &mut new_players, Some(gamepad_id));
        }

        if (false) {
            // Need to rethink this
            // I want this to be possible but will probably need some interaction setup

            let mut input = Input::None;
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_LEFT) {
                input = Input::Left;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_RIGHT) {
                input = Input::Right;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_UP) {
                input = Input::Up;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_DOWN) {
                input = Input::Down;
            }
            Self::process_input(&mut self.controller_b_players[gamepad_id as usize], input, &mut player_inputs, timeline, players_local, outfit_switchers, &mut new_players, Some(gamepad_id));
        }
    }
    */
}

static mut g_hack_last_steam_input_toggle_t: i32 = 0;

pub fn toggle_pause() -> bool {
    if key_pressed(raylib_sys::KeyboardKey::KEY_ESCAPE) {
        return true;
    }

    #[cfg(feature = "steam")]
    {
        if using_steam_input() {
            // @Hack, when we go to the pause menu we change actionstates
            // This means that we will immediately trigger this again. 
            // Hack around by using a timer.

            unsafe {
                if (crate::steam::g_t - g_hack_last_steam_input_toggle_t > 5) {
                    if crate::steam::read_menu_input().0 == MenuInput::ReturnToGame {
                        g_hack_last_steam_input_toggle_t = crate::steam::g_t;
                        return true;
                    }

                    if (crate::steam::game_pause_pressed()) {
                        g_hack_last_steam_input_toggle_t = crate::steam::g_t;
                        return true;
                    }
                }
            }
        }
    }

    // @TODO @incomplete handle non-steam input for pause.

    false
}

pub fn goto_next_title() -> bool {
    unsafe {
        if raylib_sys::GetKeyPressed() != 0 {
            return true;
        }
    }

    #[cfg(feature = "steam")]
    {
        if using_steam_input() {
            return crate::steam::read_menu_input().0 != MenuInput::None;
        }
    }

    for i in 0..4 {
        let gamepad_id = i as i32;
        if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_UP) {
            return true;
        }
        if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_LEFT) {
            return true;
        }
        if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_DOWN) {
            return true;
        }
        if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_RIGHT) {
            return true;
        }

        if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_LEFT) {
            return true;
        }
        if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_RIGHT) {
            return true;
        }
        if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_UP) {
            return true;
        }
        if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_DOWN) {
            return true;
        }
    }

    false
}