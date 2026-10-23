use macroquad::prelude::*;
use std::collections::VecDeque;
use std::fs;
use std::io;
use std::path::Path;

const GRID: i32 = 24;
const CELL: f32 = 25.0;
const WIDTH: f32 = GRID as f32 * CELL;
const HEIGHT: f32 = GRID as f32 * CELL;
const LEADERBOARD_FILE: &str = "leaderboard.txt";
const MAX_USERNAME_LENGTH: usize = 20;
const MAX_LEADERBOARD_ENTRIES: usize = 5;

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

struct ScoreEntry {
    username: String,
    score: u32,
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
        window_width: WIDTH as i32,
        window_height: HEIGHT as i32,
        ..Default::default()
    }
}

fn random_food(snake: &VecDeque<Pos>) -> Pos {
    loop {
        let food = Pos {
            x: macroquad::rand::gen_range(0, GRID),
            y: macroquad::rand::gen_range(0, GRID),
        };

        if !snake.contains(&food) {
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
            let (username, score) = line.split_once('\t')?;
            let score = score.parse::<u32>().ok()?;
            if username.is_empty() {
                return None;
            }
            Some(ScoreEntry {
                username: username.to_owned(),
                score,
            })
        })
        .collect();
    sort_and_trim_leaderboard(&mut entries);
    Ok(entries)
}

fn save_leaderboard(path: &Path, entries: &[ScoreEntry]) -> io::Result<()> {
    let contents = entries
        .iter()
        .map(|entry| format!("{}\t{}\n", entry.username, entry.score))
        .collect::<String>();
    fs::write(path, contents)
}

fn sort_and_trim_leaderboard(entries: &mut Vec<ScoreEntry>) {
    entries.sort_by(|a, b| b.score.cmp(&a.score));
    entries.truncate(MAX_LEADERBOARD_ENTRIES);
}

fn fresh_snake() -> VecDeque<Pos> {
    VecDeque::from([
        Pos { x: 12, y: 12 },
        Pos { x: 11, y: 12 },
        Pos { x: 10, y: 12 },
    ])
}

fn draw_centered(text: &str, y: f32, font_size: f32, color: Color) {
    let width = measure_text(text, None, font_size as u16, 1.0).width;
    draw_text(text, (WIDTH - width) / 2.0, y, font_size, color);
}

fn draw_leaderboard(entries: &[ScoreEntry], y: f32) {
    draw_centered("TOP 5", y, 24.0, YELLOW);
    if entries.is_empty() {
        draw_centered("No scores yet", y + 30.0, 18.0, LIGHTGRAY);
        return;
    }

    for (index, entry) in entries.iter().enumerate() {
        let line = format!("{}. {}  -  {}", index + 1, entry.username, entry.score);
        draw_centered(&line, y + 30.0 + index as f32 * 25.0, 18.0, WHITE);
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let leaderboard_path = Path::new(LEADERBOARD_FILE);
    let (mut leaderboard, mut leaderboard_error) = match load_leaderboard(leaderboard_path) {
        Ok(entries) => (entries, None),
        Err(error) => (
            Vec::new(),
            Some(format!("Could not load leaderboard: {error}")),
        ),
    };
    let mut can_save_leaderboard = leaderboard_error.is_none();

    let mut screen = Screen::Start;
    let mut username = String::new();
    let mut difficulty = Difficulty::Normal;
    let mut snake = fresh_snake();
    let mut direction = Pos { x: 1, y: 0 };
    let mut food = random_food(&snake);
    let mut score = 0;
    let mut timer = 0.0;
    let mut score_recorded = false;

    loop {
        let dt = get_frame_time();

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
                if is_key_pressed(KeyCode::Enter) && !username.trim().is_empty() {
                    snake = fresh_snake();
                    direction = Pos { x: 1, y: 0 };
                    food = random_food(&snake);
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

                        let eating = next == food;
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
                            if eating {
                                score += 1;
                                food = random_food(&snake);
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
                    });
                    sort_and_trim_leaderboard(&mut leaderboard);
                    score_recorded = true;

                    if can_save_leaderboard {
                        if let Err(error) = save_leaderboard(leaderboard_path, &leaderboard) {
                            leaderboard_error =
                                Some(format!("Could not save leaderboard: {error}"));
                            can_save_leaderboard = false;
                        }
                    }
                }
                if is_key_pressed(KeyCode::R) {
                    screen = Screen::Start;
                }
            }
        }

        clear_background(Color::from_rgba(18, 22, 30, 255));

        if screen == Screen::Start {
            draw_centered("RUST SNAKE", 90.0, 48.0, LIME);
            draw_centered("Enter username (max 20 characters)", 150.0, 20.0, WHITE);
            let name_display = if username.is_empty() {
                "_".to_owned()
            } else {
                format!("{username}_")
            };
            draw_centered(&name_display, 185.0, 24.0, YELLOW);
            draw_centered("Choose difficulty:  <  /  >", 235.0, 20.0, WHITE);
            draw_centered(difficulty.name(), 270.0, 26.0, ORANGE);
            draw_centered("Press ENTER to start", 315.0, 20.0, LIGHTGRAY);
            draw_leaderboard(&leaderboard, 375.0);
            if let Some(error) = &leaderboard_error {
                draw_centered(error, HEIGHT - 15.0, 14.0, RED);
            }
        } else {
            for i in 0..=GRID {
                let p = i as f32 * CELL;
                draw_line(p, 0.0, p, HEIGHT, 1.0, Color::from_rgba(35, 42, 52, 255));
                draw_line(0.0, p, WIDTH, p, 1.0, Color::from_rgba(35, 42, 52, 255));
            }

            draw_rectangle(
                food.x as f32 * CELL + 3.0,
                food.y as f32 * CELL + 3.0,
                CELL - 6.0,
                CELL - 6.0,
                RED,
            );
            for (index, part) in snake.iter().enumerate() {
                let color = if index == 0 { LIME } else { GREEN };
                draw_rectangle(
                    part.x as f32 * CELL + 2.0,
                    part.y as f32 * CELL + 2.0,
                    CELL - 4.0,
                    CELL - 4.0,
                    color,
                );
            }

            draw_text(
                &format!("{username}  Score: {score}"),
                10.0,
                22.0,
                22.0,
                WHITE,
            );
            draw_text(
                &format!("{} | P/Esc: pause", difficulty.name()),
                10.0,
                HEIGHT - 10.0,
                16.0,
                LIGHTGRAY,
            );
        }

        if screen == Screen::Paused || screen == Screen::GameOver {
            draw_rectangle(0.0, 0.0, WIDTH, HEIGHT, Color::from_rgba(0, 0, 0, 190));
            if screen == Screen::Paused {
                draw_centered("PAUSED", HEIGHT / 2.0 - 15.0, 40.0, YELLOW);
                draw_centered("Press P or ESC to resume", HEIGHT / 2.0 + 25.0, 20.0, WHITE);
            } else {
                draw_centered("GAME OVER", HEIGHT / 2.0 - 55.0, 40.0, RED);
                draw_centered(&format!("Score: {score}"), HEIGHT / 2.0 - 15.0, 22.0, WHITE);
                draw_centered(
                    "Press R to return to start",
                    HEIGHT / 2.0 + 25.0,
                    20.0,
                    WHITE,
                );
                if let Some(error) = &leaderboard_error {
                    draw_centered(error, HEIGHT / 2.0 + 60.0, 14.0, RED);
                }
            }
        }

        next_frame().await;
    }
}
