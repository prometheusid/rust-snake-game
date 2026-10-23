#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use macroquad::prelude::*;
use std::collections::VecDeque;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const GRID: i32 = 24;
const CELL: f32 = 25.0;
const BOARD_SIZE: f32 = GRID as f32 * CELL;
const WINDOW_WIDTH: f32 = BOARD_SIZE;
const WINDOW_HEIGHT: f32 = BOARD_SIZE * 2.0;
const BOARD_X: f32 = (WINDOW_WIDTH - BOARD_SIZE) / 2.0;
const BOARD_Y: f32 = 0.0;
const PANEL_Y: f32 = BOARD_SIZE;
const START_LEFT_BUTTON: Rect = Rect::new(140.0, 255.0, 50.0, 42.0);
const START_RIGHT_BUTTON: Rect = Rect::new(410.0, 255.0, 50.0, 42.0);
const LEADERBOARD_FILE: &str = "leaderboard.txt";
const MAX_USERNAME_LENGTH: usize = 20;
const MAX_LEADERBOARD_ENTRIES: usize = 10;

#[derive(Clone, Copy, PartialEq)]
struct Pos {
    x: i32,
    y: i32,
}

#[derive(Clone, Copy)]
enum Difficulty {
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    fn all() -> [Self; 3] {
        [Self::Easy, Self::Normal, Self::Hard]
    }

    fn name(self) -> &'static str {
        match self {
            Self::Easy => "Easy",
            Self::Normal => "Normal",
            Self::Hard => "Hard",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Easy => 0,
            Self::Normal => 1,
            Self::Hard => 2,
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        match name {
            "Easy" => Some(Self::Easy),
            "Normal" => Some(Self::Normal),
            "Hard" => Some(Self::Hard),
            _ => None,
        }
    }

    fn step_interval(self) -> f32 {
        match self {
            Self::Easy => 0.18,
            Self::Normal => 0.12,
            Self::Hard => 0.075,
        }
    }

    fn change(self, offset: i32) -> Self {
        let levels = Self::all();
        let index = match self {
            Self::Easy => 0,
            Self::Normal => 1,
            Self::Hard => 2,
        };
        levels[(index as i32 + offset).rem_euclid(levels.len() as i32) as usize]
    }
}

#[derive(Clone)]
struct ScoreEntry {
    username: String,
    score: u32,
    difficulty: Difficulty,
}

#[derive(PartialEq)]
enum Screen {
    Start,
    Playing,
    Paused,
    GameOver,
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Rust Snake".to_owned(),
        window_width: 540,
        window_height: 960,
        window_resizable: true,
        ..Default::default()
    }
}

fn logical_viewport() -> (f32, f32, f32) {
    let scale = (screen_width() / WINDOW_WIDTH).min(screen_height() / WINDOW_HEIGHT);
    let viewport_width = WINDOW_WIDTH * scale;
    let viewport_height = WINDOW_HEIGHT * scale;
    (
        (screen_width() - viewport_width) / 2.0,
        (screen_height() - viewport_height) / 2.0,
        scale,
    )
}

fn game_camera() -> Camera2D {
    let (left, top, scale) = logical_viewport();
    Camera2D {
        target: vec2(WINDOW_WIDTH / 2.0, WINDOW_HEIGHT / 2.0),
        zoom: vec2(2.0 / WINDOW_WIDTH, 2.0 / WINDOW_HEIGHT),
        viewport: Some((
            left.round() as i32,
            top.round() as i32,
            (WINDOW_WIDTH * scale).round() as i32,
            (WINDOW_HEIGHT * scale).round() as i32,
        )),
        ..Default::default()
    }
}

fn logical_mouse_position() -> Vec2 {
    let (left, top, scale) = logical_viewport();
    let (mouse_x, mouse_y) = mouse_position();
    vec2((mouse_x - left) / scale, (mouse_y - top) / scale)
}

fn random_food(snake: &VecDeque<Pos>, other_food: Option<Pos>) -> Pos {
    loop {
        let food = Pos {
            x: macroquad::rand::gen_range(0, GRID),
            y: macroquad::rand::gen_range(0, GRID),
        };

        if !snake.contains(&food) && Some(food) != other_food {
            return food;
        }
    }
}

fn load_leaderboard(path: &Path) -> io::Result<Vec<ScoreEntry>> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };

    let mut entries: Vec<ScoreEntry> = contents
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let username = fields.next()?;
            let second_field = fields.next()?;
            if username.is_empty() {
                return None;
            }

            let (difficulty, score) = match fields.next() {
                Some(score) => (Difficulty::from_name(second_field)?, score),
                None => (Difficulty::Normal, second_field),
            };
            let score = score.parse::<u32>().ok()?;

            Some(ScoreEntry {
                username: username.to_owned(),
                score,
                difficulty,
            })
        })
        .collect();
    sort_and_trim_leaderboard(&mut entries);
    Ok(entries)
}

fn save_leaderboard(path: &Path, entries: &[ScoreEntry]) -> io::Result<()> {
    let contents = entries
        .iter()
        .map(|entry| {
            format!(
                "{}\t{}\t{}\n",
                entry.username,
                entry.difficulty.name(),
                entry.score
            )
        })
        .collect::<String>();
    fs::write(path, contents)
}

fn leaderboard_file_path() -> io::Result<PathBuf> {
    let executable = std::env::current_exe()?;
    let executable_directory = executable.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "the executable path has no parent directory",
        )
    })?;
    Ok(executable_directory.join(LEADERBOARD_FILE))
}

fn sort_and_trim_leaderboard(entries: &mut Vec<ScoreEntry>) {
    entries.sort_by(|a, b| {
        b.difficulty
            .rank()
            .cmp(&a.difficulty.rank())
            .then_with(|| b.score.cmp(&a.score))
    });
    entries.truncate(MAX_LEADERBOARD_ENTRIES);
}

fn fresh_snake() -> VecDeque<Pos> {
    VecDeque::from([
        Pos { x: 12, y: 12 },
        Pos { x: 11, y: 12 },
        Pos { x: 10, y: 12 },
    ])
}

fn rainbow_color(segment: usize, time: f64) -> Color {
    let hue = (segment as f64 * 0.18 + time * 0.8).rem_euclid(6.0);
    let chroma = 1.0_f32;
    let secondary = (1.0 - (hue.rem_euclid(2.0) - 1.0).abs()) as f32;
    let (red, green, blue) = match hue as u8 {
        0 => (chroma, secondary, 0.0),
        1 => (secondary, chroma, 0.0),
        2 => (0.0, chroma, secondary),
        3 => (0.0, secondary, chroma),
        4 => (secondary, 0.0, chroma),
        _ => (chroma, 0.0, secondary),
    };
    Color::new(red, green, blue, 1.0)
}

fn draw_centered_in_rect(text: &str, rect: Rect, font_size: f32, color: Color) {
    let dimensions = measure_text(text, None, font_size as u16, 1.0);
    draw_text(
        text,
        rect.x + (rect.w - dimensions.width) / 2.0,
        rect.y + (rect.h + dimensions.height) / 2.0,
        font_size,
        color,
    );
}

fn draw_centered_in_panel(text: &str, y: f32, font_size: f32, color: Color) {
    let width = measure_text(text, None, font_size as u16, 1.0).width;
    draw_text(text, (WINDOW_WIDTH - width) / 2.0, y, font_size, color);
}

fn draw_start_leaderboard(entries: &[ScoreEntry], error: Option<&str>) {
    draw_centered_in_rect(
        "TOP 5 LEADERBOARD",
        Rect::new(0.0, 380.0, WINDOW_WIDTH, 42.0),
        31.0,
        YELLOW,
    );

    if entries.is_empty() {
        draw_centered_in_rect(
            "No scores yet",
            Rect::new(0.0, 425.0, WINDOW_WIDTH, 36.0),
            23.0,
            LIGHTGRAY,
        );
    } else {
        for (index, entry) in entries.iter().take(5).enumerate() {
            let line = format!(
                "{}. {}  -  {}  ({})",
                index + 1,
                entry.username,
                entry.score,
                entry.difficulty.name()
            );
            draw_centered_in_rect(
                &line,
                Rect::new(0.0, 425.0 + index as f32 * 40.0, WINDOW_WIDTH, 36.0),
                23.0,
                WHITE,
            );
        }
    }

    if let Some(error) = error {
        draw_centered_in_rect(
            error,
            Rect::new(0.0, WINDOW_HEIGHT - 35.0, WINDOW_WIDTH, 24.0),
            16.0,
            RED,
        );
    }
}

fn draw_side_panel(
    username: &str,
    difficulty: Difficulty,
    score: u32,
    entries: &[ScoreEntry],
    error: Option<&str>,
) {
    draw_rectangle(
        0.0,
        PANEL_Y,
        WINDOW_WIDTH,
        WINDOW_HEIGHT - PANEL_Y,
        Color::from_rgba(25, 31, 42, 255),
    );
    draw_line(
        20.0,
        PANEL_Y,
        WINDOW_WIDTH - 20.0,
        PANEL_Y,
        2.0,
        Color::from_rgba(70, 82, 100, 255),
    );
    draw_centered_in_panel("PLAYER", 655.0, 23.0, LIGHTGRAY);
    let displayed_username = if username.is_empty() {
        "Enter username"
    } else {
        username
    };
    draw_centered_in_panel(displayed_username, 688.0, 34.0, YELLOW);
    draw_centered_in_panel(difficulty.name(), 720.0, 28.0, ORANGE);
    draw_centered_in_panel(&format!("LIVE SCORE  {score}"), 760.0, 44.0, LIME);
    draw_line(
        30.0,
        785.0,
        WINDOW_WIDTH - 30.0,
        785.0,
        1.0,
        Color::from_rgba(70, 82, 100, 255),
    );
    draw_centered_in_panel("LIVE LEADERBOARD - TOP 10", 820.0, 29.0, YELLOW);
    if entries.is_empty() {
        draw_centered_in_panel("No scores yet", 853.0, 23.0, LIGHTGRAY);
    } else {
        for (index, entry) in entries.iter().enumerate() {
            let line = format!(
                "{:>2}. {:<20} {:>5}  {}",
                index + 1,
                entry.username,
                entry.score,
                entry.difficulty.name()
            );
            draw_text(&line, 28.0, 853.0 + index as f32 * 30.0, 22.0, WHITE);
        }
    }

    draw_centered_in_panel("WASD: move    P/Esc: pause", 1150.0, 22.0, LIGHTGRAY);

    if let Some(error) = error {
        draw_centered_in_panel(error, WINDOW_HEIGHT - 28.0, 15.0, RED);
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let leaderboard_path = leaderboard_file_path();
    let (mut leaderboard, mut leaderboard_error) = match &leaderboard_path {
        Ok(path) => match load_leaderboard(path) {
            Ok(entries) => (entries, None),
            Err(error) => (
                Vec::new(),
                Some(format!("Could not load leaderboard: {error}")),
            ),
        },
        Err(error) => (
            Vec::new(),
            Some(format!("Could not locate leaderboard file: {error}")),
        ),
    };
    let mut can_save_leaderboard = leaderboard_error.is_none();

    let mut screen = Screen::Start;
    let mut username = String::new();
    let mut difficulty = Difficulty::Normal;
    let mut snake = fresh_snake();
    let mut direction = Pos { x: 1, y: 0 };
    let mut food = random_food(&snake, None);
    let mut bonus_food = None;
    let mut normal_food_eaten = 0;
    let mut score = 0;
    let mut timer = 0.0;
    let mut score_recorded = false;
    let mut score_popup_timer = 0.0;
    let mut score_popup_position = Vec2::ZERO;
    let mut score_popup_points = 1;

    loop {
        let dt = get_frame_time();
        if screen == Screen::Playing {
            score_popup_timer = (score_popup_timer - dt).max(0.0);
        }

        if screen != Screen::Start {
            while get_char_pressed().is_some() {}
        }

        match screen {
            Screen::Start => {
                while let Some(character) = get_char_pressed() {
                    if !character.is_control() && username.chars().count() < MAX_USERNAME_LENGTH {
                        username.push(character);
                    }
                }

                if is_key_pressed(KeyCode::Backspace) {
                    username.pop();
                }
                if is_key_pressed(KeyCode::Left) {
                    difficulty = difficulty.change(-1);
                }
                if is_key_pressed(KeyCode::Right) {
                    difficulty = difficulty.change(1);
                }
                if is_mouse_button_pressed(MouseButton::Left) {
                    let mouse_position = logical_mouse_position();
                    if START_LEFT_BUTTON.contains(mouse_position) {
                        difficulty = difficulty.change(-1);
                    } else if START_RIGHT_BUTTON.contains(mouse_position) {
                        difficulty = difficulty.change(1);
                    }
                }
                if is_key_pressed(KeyCode::Enter) && !username.trim().is_empty() {
                    snake = fresh_snake();
                    direction = Pos { x: 1, y: 0 };
                    bonus_food = None;
                    normal_food_eaten = 0;
                    food = random_food(&snake, bonus_food);
                    score = 0;
                    timer = 0.0;
                    score_recorded = false;
                    screen = Screen::Playing;
                }
            }
            Screen::Playing => {
                if is_key_pressed(KeyCode::P) || is_key_pressed(KeyCode::Escape) {
                    screen = Screen::Paused;
                } else {
                    if is_key_pressed(KeyCode::W) && direction.y != 1 {
                        direction = Pos { x: 0, y: -1 };
                    }
                    if is_key_pressed(KeyCode::S) && direction.y != -1 {
                        direction = Pos { x: 0, y: 1 };
                    }
                    if is_key_pressed(KeyCode::A) && direction.x != 1 {
                        direction = Pos { x: -1, y: 0 };
                    }
                    if is_key_pressed(KeyCode::D) && direction.x != -1 {
                        direction = Pos { x: 1, y: 0 };
                    }

                    timer += dt;
                    if timer >= difficulty.step_interval() {
                        timer = 0.0;
                        let head = snake[0];
                        let mut next = Pos {
                            x: head.x + direction.x,
                            y: head.y + direction.y,
                        };

                        if next.x < 0 {
                            next.x = GRID - 1;
                        } else if next.x >= GRID {
                            next.x = 0;
                        }
                        if next.y < 0 {
                            next.y = GRID - 1;
                        } else if next.y >= GRID {
                            next.y = 0;
                        }

                        let eating_bonus = bonus_food == Some(next);
                        let eating_normal = next == food;
                        let eating = eating_normal || eating_bonus;
                        let body_collision = snake
                            .iter()
                            .take(if eating {
                                snake.len()
                            } else {
                                snake.len().saturating_sub(1)
                            })
                            .any(|part| *part == next);

                        if body_collision {
                            screen = Screen::GameOver;
                        } else {
                            snake.push_front(next);
                            if eating_normal {
                                normal_food_eaten += 1;
                                score += 1;
                                score_popup_timer = 0.8;
                                score_popup_points = 1;
                                score_popup_position = Vec2::new(
                                    BOARD_X + next.x as f32 * CELL + CELL / 2.0,
                                    BOARD_Y + next.y as f32 * CELL,
                                );
                                food = random_food(&snake, bonus_food);
                                if normal_food_eaten % 10 == 0 {
                                    bonus_food = Some(random_food(&snake, Some(food)));
                                }
                            } else if eating_bonus {
                                score += 5;
                                score_popup_timer = 0.8;
                                score_popup_points = 5;
                                score_popup_position = Vec2::new(
                                    BOARD_X + next.x as f32 * CELL + CELL / 2.0,
                                    BOARD_Y + next.y as f32 * CELL,
                                );
                                bonus_food = None;
                            } else {
                                snake.pop_back();
                            }
                        }
                    }
                }
            }
            Screen::Paused => {
                if is_key_pressed(KeyCode::P) || is_key_pressed(KeyCode::Escape) {
                    screen = Screen::Playing;
                }
            }
            Screen::GameOver => {
                if !score_recorded {
                    leaderboard.push(ScoreEntry {
                        username: username.clone(),
                        score,
                        difficulty,
                    });
                    sort_and_trim_leaderboard(&mut leaderboard);
                    score_recorded = true;

                    if can_save_leaderboard {
                        if let Ok(path) = &leaderboard_path {
                            if let Err(error) = save_leaderboard(path, &leaderboard) {
                                leaderboard_error =
                                    Some(format!("Could not save leaderboard: {error}"));
                                can_save_leaderboard = false;
                            }
                        }
                    }
                }
                if is_key_pressed(KeyCode::R) {
                    screen = Screen::Start;
                }
            }
        }

        set_default_camera();
        clear_background(Color::from_rgba(7, 9, 14, 255));
        set_camera(&game_camera());
        draw_rectangle(
            0.0,
            0.0,
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            Color::from_rgba(18, 22, 30, 255),
        );

        let mut displayed_leaderboard = leaderboard.clone();
        if matches!(screen, Screen::Playing | Screen::Paused) {
            displayed_leaderboard.push(ScoreEntry {
                username: username.clone(),
                score,
                difficulty,
            });
            sort_and_trim_leaderboard(&mut displayed_leaderboard);
        }

        if screen == Screen::Start {
            draw_centered_in_rect(
                "RUST SNAKE",
                Rect::new(0.0, 60.0, WINDOW_WIDTH, 70.0),
                58.0,
                LIME,
            );
            draw_centered_in_rect(
                "Enter username (max 20 characters)",
                Rect::new(0.0, 135.0, WINDOW_WIDTH, 35.0),
                25.0,
                WHITE,
            );
            let name_display = if username.is_empty() {
                "_".to_owned()
            } else {
                format!("{username}_")
            };
            draw_centered_in_rect(
                &name_display,
                Rect::new(0.0, 170.0, WINDOW_WIDTH, 35.0),
                32.0,
                YELLOW,
            );
            draw_centered_in_rect(
                "Choose difficulty",
                Rect::new(0.0, 215.0, WINDOW_WIDTH, 30.0),
                27.0,
                WHITE,
            );
            draw_rectangle(
                START_LEFT_BUTTON.x,
                START_LEFT_BUTTON.y,
                START_LEFT_BUTTON.w,
                START_LEFT_BUTTON.h,
                DARKGRAY,
            );
            draw_centered_in_rect("<", START_LEFT_BUTTON, 32.0, WHITE);
            draw_centered_in_rect(
                difficulty.name(),
                Rect::new(190.0, 255.0, 220.0, 42.0),
                32.0,
                ORANGE,
            );
            draw_rectangle(
                START_RIGHT_BUTTON.x,
                START_RIGHT_BUTTON.y,
                START_RIGHT_BUTTON.w,
                START_RIGHT_BUTTON.h,
                DARKGRAY,
            );
            draw_centered_in_rect(">", START_RIGHT_BUTTON, 32.0, WHITE);
            draw_centered_in_rect(
                "Use arrow keys or click the buttons",
                Rect::new(0.0, 300.0, WINDOW_WIDTH, 28.0),
                22.0,
                LIGHTGRAY,
            );
            draw_centered_in_rect(
                "Press ENTER to start",
                Rect::new(0.0, 335.0, WINDOW_WIDTH, 32.0),
                26.0,
                LIGHTGRAY,
            );
            draw_start_leaderboard(&leaderboard, leaderboard_error.as_deref());
        } else {
            draw_rectangle(
                BOARD_X,
                BOARD_Y,
                BOARD_SIZE,
                BOARD_SIZE,
                Color::from_rgba(18, 22, 30, 255),
            );
            for i in 0..=GRID {
                let p = i as f32 * CELL;
                draw_line(
                    BOARD_X + p,
                    BOARD_Y,
                    BOARD_X + p,
                    BOARD_Y + BOARD_SIZE,
                    1.0,
                    Color::from_rgba(35, 42, 52, 255),
                );
                draw_line(
                    BOARD_X,
                    BOARD_Y + p,
                    BOARD_X + BOARD_SIZE,
                    BOARD_Y + p,
                    1.0,
                    Color::from_rgba(35, 42, 52, 255),
                );
            }

            draw_rectangle(
                BOARD_X + food.x as f32 * CELL + 3.0,
                BOARD_Y + food.y as f32 * CELL + 3.0,
                CELL - 6.0,
                CELL - 6.0,
                RED,
            );
            if let Some(bonus) = bonus_food {
                draw_circle(
                    BOARD_X + bonus.x as f32 * CELL + CELL / 2.0,
                    BOARD_Y + bonus.y as f32 * CELL + CELL / 2.0,
                    CELL * 0.4,
                    GOLD,
                );
                draw_text(
                    "+5",
                    BOARD_X + bonus.x as f32 * CELL + 2.0,
                    BOARD_Y + bonus.y as f32 * CELL - 2.0,
                    14.0,
                    WHITE,
                );
            }
            for (index, part) in snake.iter().enumerate() {
                let color = rainbow_color(index, get_time());
                draw_rectangle(
                    BOARD_X + part.x as f32 * CELL + 2.0,
                    BOARD_Y + part.y as f32 * CELL + 2.0,
                    CELL - 4.0,
                    CELL - 4.0,
                    color,
                );
            }
        }

        if score_popup_timer > 0.0 && matches!(screen, Screen::Playing | Screen::Paused) {
            let progress = 1.0 - score_popup_timer / 0.8;
            let popup_y = score_popup_position.y - 16.0 - progress * 28.0;
            let alpha = (score_popup_timer / 0.8).clamp(0.0, 1.0);
            draw_text(
                &format!("+{score_popup_points}"),
                score_popup_position.x - 12.0,
                popup_y,
                32.0,
                Color::new(0.7, 1.0, 0.2, alpha),
            );
        }

        if screen == Screen::Paused || screen == Screen::GameOver {
            draw_rectangle(
                BOARD_X,
                BOARD_Y,
                BOARD_SIZE,
                BOARD_SIZE,
                Color::from_rgba(0, 0, 0, 190),
            );
            if screen == Screen::Paused {
                draw_centered_in_rect(
                    "PAUSED",
                    Rect::new(BOARD_X, BOARD_Y + 250.0, BOARD_SIZE, 60.0),
                    48.0,
                    YELLOW,
                );
                draw_centered_in_rect(
                    "Press P or ESC to resume",
                    Rect::new(BOARD_X, BOARD_Y + 305.0, BOARD_SIZE, 40.0),
                    25.0,
                    WHITE,
                );
            } else {
                draw_centered_in_rect(
                    "GAME OVER",
                    Rect::new(BOARD_X, BOARD_Y + 225.0, BOARD_SIZE, 60.0),
                    48.0,
                    RED,
                );
                draw_centered_in_rect(
                    &format!("Score: {score}"),
                    Rect::new(BOARD_X, BOARD_Y + 275.0, BOARD_SIZE, 40.0),
                    28.0,
                    WHITE,
                );
                draw_centered_in_rect(
                    "Press R to return to start",
                    Rect::new(BOARD_X, BOARD_Y + 320.0, BOARD_SIZE, 40.0),
                    25.0,
                    WHITE,
                );
            }
        }

        if screen != Screen::Start {
            draw_side_panel(
                &username,
                difficulty,
                score,
                &displayed_leaderboard,
                leaderboard_error.as_deref(),
            );
        }

        next_frame().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn leaderboard_ranks_difficulty_before_score() {
        let mut entries = vec![
            ScoreEntry {
                username: "EasyHigh".to_owned(),
                score: 100,
                difficulty: Difficulty::Easy,
            },
            ScoreEntry {
                username: "NormalLow".to_owned(),
                score: 1,
                difficulty: Difficulty::Normal,
            },
            ScoreEntry {
                username: "HardLow".to_owned(),
                score: 0,
                difficulty: Difficulty::Hard,
            },
            ScoreEntry {
                username: "HardHigh".to_owned(),
                score: 20,
                difficulty: Difficulty::Hard,
            },
        ];

        sort_and_trim_leaderboard(&mut entries);

        let ranked_users: Vec<&str> = entries
            .iter()
            .map(|entry| entry.username.as_str())
            .collect();
        assert_eq!(
            ranked_users,
            ["HardHigh", "HardLow", "NormalLow", "EasyHigh"]
        );
    }

    #[test]
    fn leaderboard_file_path_is_next_to_the_executable() {
        let executable = std::env::current_exe().unwrap();
        let expected = executable.parent().unwrap().join(LEADERBOARD_FILE);

        assert_eq!(leaderboard_file_path().unwrap(), expected);
    }

    #[test]
    fn missing_leaderboard_is_empty_and_saved_scores_can_be_reloaded() {
        let unique_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "rust-snake-leaderboard-{}-{unique_id}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join(LEADERBOARD_FILE);

        assert!(load_leaderboard(&path).unwrap().is_empty());

        let entries = vec![ScoreEntry {
            username: "Portable".to_owned(),
            score: 42,
            difficulty: Difficulty::Hard,
        }];
        save_leaderboard(&path, &entries).unwrap();
        let loaded = load_leaderboard(&path).unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].username, "Portable");
        assert_eq!(loaded[0].score, 42);
        assert_eq!(loaded[0].difficulty.name(), "Hard");

        fs::remove_dir_all(directory).unwrap();
    }
}
