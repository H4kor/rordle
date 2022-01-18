extern crate termion;

use clap::{App, Arg};
use rand::prelude::*;
use std::fs::File;
use std::io::Read;
use std::io::{stdin, stdout, Write};
use termion::color;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

#[derive(Debug, Eq, PartialEq, Clone)]
enum HitInfo {
    Hit,
    Contains,
    Miss,
    None,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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
    current_guess: String,
    word: String,
    max_tries: u16,
    last_error: Option<GameError>,
    any_word: bool,
}

impl GameState {
    pub fn new(word: String, valid_words: Vec<String>, any_word: bool) -> GameState {
        GameState {
            valid_words,
            guesses: Vec::new(),
            current_guess: String::new(),
            word,
            max_tries: 6,
            last_error: None,
            any_word,
        }
    }

    fn guess(&mut self, guess: String) -> Result<bool, GameError> {
        if guess.chars().count() != self.word.chars().count() {
            return Err(GameError::WrongLength);
        }
        if !self.any_word && !self.valid_words.contains(&guess) {
            return Err(GameError::InvalidWord);
        }
        self.guesses.push(guess);
        Ok(self.won())
    }

    fn set_last_error(&mut self, error: GameError) {
        self.last_error = Some(error);
    }

    fn reset_error(&mut self) {
        self.last_error = None;
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

    pub fn back(&mut self) {
        if self.current_guess.chars().count() > 0 {
            self.current_guess.pop();
        }
    }

    pub fn confirm(&mut self) {
        let result = self.guess(self.current_guess.clone());
        match result {
            Ok(_) => {
                self.reset_error();
            }
            Err(error) => {
                self.set_last_error(error);
            }
        };
        self.current_guess = String::new();
    }

    pub fn add_char(&mut self, c: char) {
        if self.current_guess.chars().count() < self.word.chars().count() {
            self.current_guess.push(c.to_lowercase().next().unwrap());
        }
    }
}

fn render_game_state(game_state: &GameState) {
    let mut stdout = stdout().into_raw_mode().unwrap();
    writeln!(stdout, "{}{}", termion::clear::All, termion::cursor::Hide).unwrap();
    let width = game_state.word.chars().count() as u16;
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
        let line_guess: String;
        if y < game_state.guesses.len() as u16 {
            line_guess = game_state.guesses[y as usize].clone();
        } else if y == game_state.guesses.len() as u16 {
            let mut curr_guess = game_state.current_guess.clone();
            while curr_guess.chars().count() < width as usize {
                curr_guess.push('_');
            }
            line_guess = curr_guess;
        } else {
            line_guess = (0..width).map(|_| "_").collect::<String>();
        }

        // get hits of line
        let line_hits: Vec<HitInfo>;
        if (y as usize) < game_state.guesses.len() {
            line_hits = game_state.get_guess_hits(y as usize);
        } else {
            line_hits = vec![HitInfo::None; width as usize];
        }

        for x in 0..width {
            // print each letter into a cell
            write!(
                stdout,
                "{}|",
                termion::cursor::Goto(m_left + x * 2, m_top + y * 2),
            )
            .unwrap();

            // set color according to hit info
            let hit_info = line_hits.get(x as usize).unwrap();
            if hit_info == &HitInfo::Hit {
                write!(
                    stdout,
                    "{}{}",
                    color::Bg(color::Green),
                    color::Fg(color::Black),
                )
                .unwrap();
            }
            if hit_info == &HitInfo::Contains {
                write!(
                    stdout,
                    "{}{}",
                    color::Bg(color::Yellow),
                    color::Fg(color::Black),
                )
                .unwrap();
            }
            if hit_info == &HitInfo::Miss {
                write!(
                    stdout,
                    "{}{}",
                    color::Bg(color::Black),
                    color::Fg(color::White),
                )
                .unwrap();
            }
            if hit_info == &HitInfo::None {
                write!(
                    stdout,
                    "{}{}",
                    color::Bg(color::Reset),
                    color::Fg(color::Reset)
                )
                .unwrap();
            }

            write!(
                stdout,
                "{}{}{}",
                line_guess.chars().nth(x as usize).unwrap(),
                color::Bg(color::Reset),
                color::Fg(color::Reset)
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
    let mut stdin = stdin().keys();
    let mut stdout = stdout().into_raw_mode().unwrap();
    'game_loop: while game_state.guesses.len() < 6 {
        render_game_state(&game_state);
        'input_loop: loop {
            let b = stdin.next().unwrap().unwrap();
            match b {
                Key::Esc => break 'game_loop,
                Key::Backspace => game_state.back(),
                Key::Char('\n') => {
                    game_state.confirm();
                    break 'input_loop;
                }
                Key::Char(c) => game_state.add_char(c),
                _ => (),
            }
            stdout.flush().unwrap();
            render_game_state(&game_state);
        }

        match game_state.last_error {
            None => {
                if game_state.won() {
                    println!("You won!");
                    break;
                }
            }
            _ => (),
        }
    }

    render_game_state(&game_state);
    writeln!(stdout, "{}", termion::cursor::Show).unwrap();
    if !game_state.won() {
        println!("You lost! The word was: {}", game_state.word);
    }
}

fn init_game(any_word: bool, word_file: Option<&str>) -> GameState {
    // load valid word list from file
    let mut words = Vec::new();
    let word;

    match word_file {
        Some(file) => {
            let mut file = File::open(file).unwrap();
            let mut contents = String::new();
            file.read_to_string(&mut contents).unwrap();
            words = contents.split('\n').map(|s| s.to_string()).collect();

            let mut rng = rand::thread_rng();
            let i = rng.gen::<usize>() % words.len();
            word = words[i].clone();
        }
        None => {
            // special list of words acceptable as solutions
            let picked_word_str = include_str!("../data/picked_words.txt");
            for line in picked_word_str.lines() {
                words.push(line.to_string().to_lowercase());
            }

            let mut rng = rand::thread_rng();
            let i = rng.gen::<usize>() % words.len();
            word = words[i].clone();

            // all other words
            let valid_word_str = include_str!("../data/valid_words.txt");
            for line in valid_word_str.lines() {
                words.push(line.to_string().to_lowercase());
            }
        }
    }

    let game_state = GameState::new(word, words, any_word);
    game_state
}

fn main() {
    let matches = App::new("Rordle")
        .version("0.2.0")
        .author("Niko Abeler <niko@rerere.org>")
        .about("A Wordle clone for the terminal")
        .arg(
            Arg::new("any-word")
                .short('a')
                .long("any-word")
                .takes_value(false)
                .help("Allow any word to be guessed"),
        )
        .arg(
            Arg::new("word-file")
                .short('w')
                .long("word-file")
                .takes_value(true)
                .help("Use a word list from a file"),
        )
        .get_matches();

    let game_state = init_game(
        matches.is_present("any-word"),
        matches.value_of("word-file"),
    );
    game_loop(game_state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_guess() {
        let mut game_state =
            super::GameState::new("hello".to_string(), vec!["hello".to_string()], false);
        let result = game_state.guess("hello".to_string());
        assert_eq!(result.unwrap(), true);
        assert_eq!(game_state.guesses.len(), 1);
        assert_eq!(game_state.guesses[0], "hello".to_string());
    }

    #[test]
    fn test_new_guess_umlaut() {
        let mut game_state =
            super::GameState::new("hello".to_string(), vec!["hällö".to_string()], false);
        let result = game_state.guess("hällö".to_string());
        assert_eq!(result.unwrap(), false);
        assert_eq!(game_state.guesses.len(), 1);
        assert_eq!(game_state.guesses[0], "hällö".to_string());
    }

    #[test]
    fn test_new_guess_miss() {
        let mut game_state = super::GameState::new(
            "hello".to_string(),
            vec!["hello".to_string(), "world".to_string()],
            false,
        );
        let result = game_state.guess("world".to_string());
        assert_eq!(result.unwrap(), false);
        assert_eq!(game_state.guesses.len(), 1);
        assert_eq!(game_state.guesses[0], "world".to_string());
    }

    #[test]
    fn test_guess_rejects_word_of_wrong_length() {
        let mut game_state =
            super::GameState::new("hello".to_string(), vec!["hello".to_string()], false);
        let result = game_state.guess("hell".to_string());
        match result {
            Err(GameError::WrongLength) => assert!(true),
            _ => assert!(false, "No error raised for wrong length"),
        }
        assert_eq!(game_state.guesses.len(), 0);
    }

    #[test]
    fn test_guess_rejects_invalid_words() {
        let mut game_state =
            super::GameState::new("hello".to_string(), vec!["hello".to_string()], false);
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
            false,
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

    #[test]
    fn test_add_char() {
        let mut game_state =
            super::GameState::new("hello".to_string(), vec!["hello".to_string()], false);
        game_state.add_char('h');
        assert_eq!(game_state.current_guess, "h".to_string());
        game_state.add_char('e');
        assert_eq!(game_state.current_guess, "he".to_string());
        game_state.add_char('l');
        assert_eq!(game_state.current_guess, "hel".to_string());
        game_state.add_char('l');
        assert_eq!(game_state.current_guess, "hell".to_string());
        game_state.add_char('o');
        assert_eq!(game_state.current_guess, "hello".to_string());
        game_state.add_char('o');
        assert_eq!(game_state.current_guess, "hello".to_string());
    }

    #[test]
    fn test_add_char_test_umlaut() {
        let mut game_state =
            super::GameState::new("hello".to_string(), vec!["hello".to_string()], false);
        game_state.add_char('Ü');
        assert_eq!(game_state.current_guess, "ü".to_string());
    }

    #[test]
    fn test_add_char_test_umlaut_length() {
        let mut game_state =
            super::GameState::new("hello".to_string(), vec!["hello".to_string()], false);
        game_state.add_char('Ü');
        game_state.add_char('Ü');
        game_state.add_char('Ü');
        game_state.add_char('Ü');
        game_state.add_char('Ü');
        assert_eq!(game_state.current_guess, "üüüüü".to_string());
    }

    #[test]
    fn test_add_char_converts_to_lowercase() {
        let mut game_state =
            super::GameState::new("hello".to_string(), vec!["hello".to_string()], false);
        game_state.add_char('H');
        assert_eq!(game_state.current_guess, "h".to_string());
        game_state.add_char('E');
        assert_eq!(game_state.current_guess, "he".to_string());
    }

    #[test]
    fn test_back() {
        let mut game_state =
            super::GameState::new("hello".to_string(), vec!["hello".to_string()], false);
        game_state.add_char('h');
        assert_eq!(game_state.current_guess, "h".to_string());
        game_state.add_char('e');
        assert_eq!(game_state.current_guess, "he".to_string());
        game_state.add_char('l');
        assert_eq!(game_state.current_guess, "hel".to_string());
        game_state.back();
        assert_eq!(game_state.current_guess, "he".to_string());
    }

    #[test]
    fn test_cofirm_with_too_few_chars() {
        let mut game_state =
            super::GameState::new("hello".to_string(), vec!["hello".to_string()], false);
        game_state.add_char('h');
        game_state.add_char('e');
        game_state.confirm();
        assert_eq!(game_state.last_error.unwrap(), GameError::WrongLength);
        assert_eq!(game_state.current_guess.len(), 0);
        assert_eq!(game_state.guesses.len(), 0);
    }

    #[test]
    fn test_cofirm_with_invalid_word() {
        let mut game_state =
            super::GameState::new("hello".to_string(), vec!["hello".to_string()], false);
        game_state.add_char('j');
        game_state.add_char('e');
        game_state.add_char('l');
        game_state.add_char('l');
        game_state.add_char('o');
        game_state.confirm();
        assert_eq!(game_state.last_error.unwrap(), GameError::InvalidWord);
        assert_eq!(game_state.current_guess.len(), 0);
        assert_eq!(game_state.guesses.len(), 0);
    }

    #[test]
    fn test_cofirm() {
        let mut game_state =
            super::GameState::new("hello".to_string(), vec!["hello".to_string()], false);
        game_state.add_char('h');
        // produce error
        game_state.confirm();
        game_state.add_char('h');
        game_state.add_char('e');
        game_state.add_char('l');
        game_state.add_char('l');
        game_state.add_char('o');
        game_state.confirm();
        assert_eq!(game_state.last_error, None);
        assert_eq!(game_state.current_guess.len(), 0);
        assert_eq!(game_state.guesses.len(), 1);
    }

    #[test]
    fn test_accepts_any_word() {
        let mut game_state = super::GameState::new(
            "hello".to_string(),
            vec!["hello".to_string(), "jolly".to_string()],
            true,
        );
        let result = game_state.guess("milli".to_string()).unwrap();
        assert_eq!(result, false);
    }

    #[test]
    fn test_rendering_with_umlaut() {
        let mut game_state =
            super::GameState::new("hello".to_string(), vec!["hello".to_string()], false);
        game_state.add_char('Ü');
        render_game_state(&game_state);
    }
    #[test]
    fn test_rendering_with_one_input() {
        let mut game_state =
            super::GameState::new("hello".to_string(), vec!["hello".to_string()], false);
        game_state.add_char('w');
        render_game_state(&game_state);
    }
}
