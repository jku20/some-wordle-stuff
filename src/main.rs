use rayon::prelude::*;
use clap::{Parser, Subcommand};

///a string for loading the guesses
const GUESSES_STR: &str = include_str!("../data/guesses.txt");
///a string for loading the solutions
const SOLUTIONS_STR: &str = include_str!("../data/solutions.txt");

///Possible marks for letters, grey, yellow, green
///to be used in the Grade sturct
#[derive(Debug, Copy, Clone, PartialEq)]
enum Mark {
    Grey,
    Yellow,
    Green,
}

impl Default for Mark {
    fn default() -> Self {
        Mark::Grey
    }
}

///Wrapper stuct for an array of marks
struct Grade {
    marks: [Mark; 5],
}

impl PartialEq for Grade {
    fn eq(&self, other: &Self) -> bool {
        self.marks
            .iter()
            .zip(other.marks.iter())
            .all(|(&u, &v)| u == v)
    }
}

//a bucket is defined as all the words which are possible given a certain guess
//to construct a bucket for a word just iterate over all possible other words
//
//words are stored in an array and therefor are given an id based on that indexing
//there is no set pattern

type WordId = usize;
type Bucket = Vec<WordId>;
type WordBank<'a> = &'a [&'a str];

///Compares the solution word to the guess word. Requires a valid solution word to run and check
///the guess against.
//maybe implement caching later
//there was subtle bug where I didn't quite replicate the right behavior
//when there are multiple of the same character not in the right place but I think I fixed it now
fn solution_compare(guess: WordId, solution: WordId, gbank: WordBank, sbank: WordBank) -> Grade {
    //check if yellow
    let mut r = [Mark::Grey, Mark::Grey, Mark::Grey, Mark::Grey, Mark::Grey];
    let g = gbank[guess];
    let s = sbank[solution];
    for (i, (a, b)) in s.chars().zip(g.chars()).enumerate() {
        if a == b {
            r[i] = Mark::Green;
        }
    }
    for (i, c) in s.chars().enumerate() {
        if r[i] == Mark::Green {
            continue;
        }
        for (j, b) in g.chars().enumerate() {
            if r[j] == Mark::Grey && c == b {
                r[j] = Mark::Yellow;
                break;
            }
        }
    }
    Grade { marks: r }
}

#[cfg(test)]
mod test {
    use crate::*;
    #[test]
    fn test_compare() {
        assert!(
            solution_compare(0, 0, &["mario"], &["slane"])
                == Grade {
                    marks: [
                        Mark::Grey, 
                        Mark::Yellow, 
                        Mark::Grey, 
                        Mark::Grey, 
                        Mark::Grey,
                    ]
                }
        );
        assert!(
            solution_compare(0, 0, &["earef"], &["maree"])
                == Grade {
                    marks: [
                        Mark::Yellow,
                        Mark::Green,
                        Mark::Green,
                        Mark::Green,
                        Mark::Grey,
                    ]
                }
        );
    }
}

///Creates a bucket for a given word given
///a bank of guesses and possible solutions.
///Note I only let buckets be composed of possible solution words.
///All compare functions require a solution word though not all will use it in which case it is
///fine ot just put in whatever.
fn bucket<T>(
    guess: WordId,
    solution: WordId,
    compare: T,
    left: &[WordId],
    gbank: WordBank,
    sbank: WordBank,
) -> Bucket
where
    T: Fn(WordId, WordId, WordBank, WordBank) -> Grade,
{
    let guess_mark = compare(guess, solution, gbank, sbank);
    let mut bkt = vec![];
    for &word in left {
        let wmark = compare(guess, word, gbank, sbank);
        if wmark == guess_mark {
            bkt.push(word);
        }
    }
    bkt
}

///Finds the word which when guessed
///will cause the largest possible bucket to be smallest
///it will return that word's ID
///This word I call an sm_word here, hence the name of the function.
fn sm_word(left: &[WordId], gbank: WordBank, sbank: WordBank) -> WordId {
    //for every possible guess, find the bucket for every word left
    //choose the guess which makes it's largest bucket for every solution word smallest
    (0..gbank.len())
        .into_par_iter()
        .min_by_key(|&guess| {
            let out = left
                .iter()
                .map(|&possible_solution| {
                    bucket(
                        guess,
                        possible_solution,
                        solution_compare,
                        left,
                        gbank,
                        sbank,
                    )
                    .len()
                })
                .max()
                .unwrap();
            //commented out, though this can be a nice thing for peace of mind when running longer
            //computations
            //if guess % 50 == 0 { eprintln!("finished guess {}", guess); }
            out
        })
        .unwrap()
}

///Starting word, guess with minimum maxmimum bucket when the inital bucket is all possible
///solution words in a standard wordle game.
///found with a brute force in about 15 minutes on a gen 8 i7
///the word is "aesir"
const START_GUESS_WORD: WordId = 113;

///The game will let you define a way to guess and a way to get the current solution.
///It requires a way to determine a guess and a way to determine how that guess will be marked.
///It provides an update function which can be used to play a turn of the game. It will return
///either Some(Bucket) contianing the current game's bucket or None if the game is over.
trait Game {
    ///Guesses a word with a certain word ID
    fn guess(&mut self) -> WordId;
    ///returns the current solution word, this can depend on the guess but should be consistant to
    ///the rules of a proper wordle game.
    fn solution(&mut self, guess: WordId) -> WordId;
    ///Updates the current game, returning a Bucket of remain words or None if the game has
    ///terminated. It will update the current game state.
    fn update(&mut self) -> Option<Bucket>;
}

///A game struct for the standard wordle game with one word held constant.
struct FixedWordle<'a> {
    ///words not eliminated
    left: Bucket,
    ///the word to be guessed
    solution: WordId,
    ///the bank of all possible solution words
    sbank: &'a [&'a str],
    ///the bank of all possible guessable words
    gbank: &'a [&'a str],
    ///whether START_GUESS_WORD should be used as a guess
    hard_code_turn: bool,
}

impl<'a> FixedWordle<'a> {
    fn with_state(
        solution: WordId,
        left: Bucket,
        sbank: &'a [&str],
        gbank: &'a [&str],
        hard_code_turn: bool,
    ) -> Self {
        Self {
            left,
            solution,
            sbank,
            gbank,
            hard_code_turn,
        }
    }
}

impl<'a> Game for FixedWordle<'a> {
    //This guess can take a long time, but shouldn't
    fn guess(&mut self) -> WordId {
        if self.hard_code_turn {
            self.hard_code_turn = false;
            START_GUESS_WORD
        } else {
            sm_word(&self.left, self.gbank, self.sbank)
        }
    }
    fn solution(&mut self, _guess: WordId) -> WordId {
        self.solution
    }
    fn update(&mut self) -> Option<Bucket> {
        let w = self.guess();
        let s = self.solution(w);
        self.left = bucket(w, s, solution_compare, &self.left, self.gbank, self.sbank);
        if self.left.len() == 1 {
            None
        } else {
            Some(self.left.clone())
        }
    }
}

///finds the the starting guess
fn get_smw() {
    let guesses = GUESSES_STR
        .split('\n')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    let solutions = SOLUTIONS_STR
        .split('\n')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    let left = (0..solutions.len()).collect::<Vec<_>>();
    let smw_id = sm_word(&left, &guesses, &solutions);
    println!("sm_bucket_word ID: {}", smw_id);
    println!("sm_bucket word: {}", guesses[smw_id]);
}

///sims a game using a greedy algo and the given solution word as the solution.
///returns the number of turns the game took. I don't think there is an off by one error in there.
fn sim_game_with_solution(solution: WordId, gbank: WordBank, sbank: WordBank) -> u16 {
    let mut game = FixedWordle::with_state(
        solution,
        (0..sbank.len()).collect::<Vec<_>>(),
        sbank,
        gbank,
        true,
    );

    let mut cur_turn = 1;
    let mut cur_game_bkt = game.update();
    while cur_game_bkt != None {
        cur_game_bkt = game.update();
        cur_turn += 1;
    }
    cur_turn + 1
}

///this find the maximum game length, pretty self explanitory
fn maximum_game_length(gbank: WordBank, sbank: WordBank) -> u16 {
    (0..sbank.len())
        .into_par_iter()
        .map(|sol| {
            let out = sim_game_with_solution(sol, gbank, sbank);
            if sol % 50 == 0 { println!("finished sol {}", sol); }
            if out > 5 { println!("out is greater than 5 and is: {}", out); }
            out
        })
        .max()
        .unwrap()
}

///pretty self explanitory, runs `maximum_game_length` on the standard guesses and solutions and
///prints out the result.
fn print_maximum_game_length() {
    //load guesses from data files
    let mut guesses = GUESSES_STR
        .split('\n')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    let solutions = SOLUTIONS_STR
        .split('\n')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    //solutions are possible guesses!
    guesses.extend_from_slice(&solutions);

    //do the thing
    let mgl = maximum_game_length(&guesses, &solutions);
    println!("maximum game length: {}", mgl);
}


#[derive(Parser)]
#[clap(version, about)]
struct Args {
    #[clap(subcommand)]
    mode: Mode,
}

#[derive(Subcommand)]
enum Mode {
    ///Finds the starter based on greedy strategy
    Starter,
    ///Finds the maximum game length based on greedy strategy
    MaxGame,
}

fn main() {
    let args = Args::parse();
    match &args.mode {
        Mode::Starter => {
            get_smw();
        }
        Mode::MaxGame => {
            print_maximum_game_length();
        }
    }
}
