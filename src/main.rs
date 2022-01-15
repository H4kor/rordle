extern crate termion;

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::{stdin, stdout, Write};
use termion::color;

#[derive(Debug, Eq, PartialEq, Clone)]
enum HitInfo {
    Hit,
    Contains,
    Miss,
    None,
}

#[derive(Debug, Clone, Copy)]
enum GameError {
    WrongLength,
    InvalidWord,
}

impl std::fmt::Display for GameError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            GameError::WrongLength => write!(f, "Word is not the correct length"),
            GameError::InvalidWord => write!(f, "Word is not valid"),
        }
    }
}

struct GameState {
    valid_words: Vec<String>,
    guesses: Vec<String>,
    word: String,
    max_tries: u16,
    last_error: Option<GameError>,
}

impl GameState {
    pub fn new(word: String, valid_words: Vec<String>) -> GameState {
        GameState {
            valid_words,
            guesses: Vec::new(),
            word,
            max_tries: 6,
            last_error: None,
        }
    }

    pub fn guess(&mut self, guess: String) -> Result<bool, GameError> {
        if guess.len() != self.word.len() {
            return Err(GameError::WrongLength);
        }
        if !self.valid_words.contains(&guess) {
            return Err(GameError::InvalidWord);
        }
        self.guesses.push(guess);
        Ok(self.won())
    }

    pub fn won(&self) -> bool {
        match self.guesses.last() {
            Some(last_guess) => last_guess == &self.word,
            None => false,
        }
    }

    pub fn get_guess_hits(&self, guess_position: usize) -> Vec<HitInfo> {
        let mut hits = Vec::new();
        let guess = self.guesses.get(guess_position).unwrap();
        for (i, c) in guess.chars().enumerate() {
            if c == self.word.chars().nth(i).unwrap() {
                hits.push(HitInfo::Hit);
            } else if self.word.contains(c) {
                hits.push(HitInfo::Contains);
            } else {
                hits.push(HitInfo::Miss);
            }
        }
        hits
    }

    pub fn set_last_error(&mut self, error: GameError) {
        self.last_error = Some(error);
    }
    pub fn reset_error(&mut self) {
        self.last_error = None;
    }
}

// get new game from player
fn request_new_world() -> String {
    // println!("Please enter a new world:");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    input.trim().to_string()
}

fn init_game() -> GameState {
    // load valid word list from file
    let mut words = Vec::new();
    let mut file = File::open("words.txt").expect("Failed to open file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read file");
    for line in contents.lines() {
        words.push(line.to_string());
    }

    let mut game_state = GameState::new("abamp".to_string(), words); // TODO: pick word from list of valid words
    game_state
}

fn render_game_state(game_state: &GameState) {
    let mut stdout = stdout();
    writeln!(stdout, "{}", termion::clear::All,).unwrap();
    let width = game_state.word.len() as u16;
    let height = game_state.max_tries as u16;
    let m_top = 4;
    let m_left = 10;
    for y in 0..height {
        write!(
            stdout,
            "{}{}",
            termion::cursor::Goto(m_left, m_top + y * 2 - 1),
            (0..(width * 2 + 1)).map(|_| "-").collect::<String>()
        )
        .unwrap();

        // get guess of line or a string of underscores
        let line_guess: String = match game_state.guesses.get(y as usize) {
            Some(guess) => guess.clone(),
            None => (0..width).map(|_| "_").collect::<String>(),
        };
        // get hits of line
        let line_hits: Vec<HitInfo>;
        if (y as usize) < game_state.guesses.len() {
            line_hits = game_state.get_guess_hits(y as usize);
        } else {
            line_hits = vec![HitInfo::None; width as usize];
        }

        for x in 0..width {
            // let color = match line_hits[x as usize] {
            //     HitInfo::Hit => color::Green,
            //     HitInfo::Contains => color::Yellow,
            //     HitInfo::Miss => color::Red,
            // };

            // print each letter into a cell
            write!(
                stdout,
                "{}|",
                termion::cursor::Goto(m_left + x * 2, m_top + y * 2),
            );

            // set color according to hit info
            let hit_info = line_hits.get(x as usize).unwrap();
            if hit_info == &HitInfo::Hit {
                write!(stdout, "{}", color::Bg(color::Green),);
            }
            if hit_info == &HitInfo::Contains {
                write!(stdout, "{}", color::Bg(color::Yellow),);
            }
            if hit_info == &HitInfo::Miss {
                write!(
                    stdout,
                    "{}{}",
                    color::Bg(color::Black),
                    color::Bg(color::White),
                );
            }
            if hit_info == &HitInfo::None {
                write!(stdout, "{}", color::Bg(color::Reset),);
            }

            write!(
                stdout,
                "{}{}",
                line_guess.chars().nth(x as usize).unwrap(),
                color::Bg(color::Reset),
            )
            .unwrap();
        }
        // close cell
        writeln!(stdout, "|").unwrap();
    }
    // print error below game board
    match game_state.last_error {
        Some(error) => {
            writeln!(
                stdout,
                "{}{}",
                termion::cursor::Goto(m_left, m_top + height * 2 + 1),
                format!("{}", error)
            )
            .unwrap();
        }
        None => (),
    }
}

fn game_loop(mut game_state: GameState) {
    while game_state.guesses.len() < 6 {
        render_game_state(&game_state);
        let guess = request_new_world();
        let result = game_state.guess(guess);
        match result {
            Ok(won) => {
                game_state.reset_error();
                if won {
                    println!("You won!");
                    break;
                }
            }
            Err(e) => {
                game_state.set_last_error(e); //println!("Error: {:?}", e);
            }
        }
    }

    render_game_state(&game_state);
    if !game_state.won() {
        println!("You lost! The word was: {}", game_state.word);
    }
}

fn main() {
    let game_state = init_game();
    game_loop(game_state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_guess() {
        let mut game_state = super::GameState::new("hello".to_string(), vec!["hello".to_string()]);
        let result = game_state.guess("hello".to_string());
        assert_eq!(result.unwrap(), true);
        assert_eq!(game_state.guesses.len(), 1);
        assert_eq!(game_state.guesses[0], "hello".to_string());
    }

    #[test]
    fn test_new_guess_miss() {
        let mut game_state = super::GameState::new(
            "hello".to_string(),
            vec!["hello".to_string(), "world".to_string()],
        );
        let result = game_state.guess("world".to_string());
        assert_eq!(result.unwrap(), false);
        assert_eq!(game_state.guesses.len(), 1);
        assert_eq!(game_state.guesses[0], "world".to_string());
    }

    #[test]
    fn test_guess_rejects_word_of_wrong_length() {
        let mut game_state = super::GameState::new("hello".to_string(), vec!["hello".to_string()]);
        let result = game_state.guess("hell".to_string());
        match result {
            Err(GameError::WrongLength) => assert!(true),
            _ => assert!(false, "No error raised for wrong length"),
        }
        assert_eq!(game_state.guesses.len(), 0);
    }

    #[test]
    fn test_guess_rejects_invalid_words() {
        let mut game_state = super::GameState::new("hello".to_string(), vec!["hello".to_string()]);
        let result = game_state.guess("jello".to_string());
        match result {
            Err(GameError::InvalidWord) => assert!(true),
            _ => assert!(false, "No error raised for invalid word"),
        }
        assert_eq!(game_state.guesses.len(), 0);
    }

    #[test]
    fn test_get_guess_hits() {
        let mut game_state = super::GameState::new(
            "hello".to_string(),
            vec!["hello".to_string(), "jolly".to_string()],
        );
        let result = game_state.guess("jolly".to_string());
        assert_eq!(result.unwrap(), false);
        let hits = game_state.get_guess_hits(0);
        assert_eq!(hits.len(), 5);
        assert_eq!(hits[0], HitInfo::Miss);
        assert_eq!(hits[1], HitInfo::Contains);
        assert_eq!(hits[2], HitInfo::Hit);
        assert_eq!(hits[3], HitInfo::Hit);
        assert_eq!(hits[4], HitInfo::Miss);
    }
}
