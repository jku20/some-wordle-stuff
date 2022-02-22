use rayon::prelude::*;

const GUESSES_STR: &str = include_str!("../data/guesses.txt");
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

///Wrapper stuct for marks
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

//maybe represent this more smartly later
type WordId = usize;
type Bucket = Vec<WordId>;
type WordBank<'a> = &'a [&'a str];

///Compares the solution word to the guess word. Requires a valid solution word to run and check
///the guess against.
//maybe implement caching later
//subtle bug where I don't quite replicate the right behavior
//when there are multiple of the same character not in the right place
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
                    marks: [Mark::Grey, Mark::Yellow, Mark::Grey, Mark::Grey, Mark::Grey]
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
                        Mark::Grey
                    ]
                }
        );
    }
}

///Creates a bucket for a given word given
///a bank of guesses and possible solutions.
///note I only let buckets be composed of possible solution words
///this function also updates a used array
///Note that not all compare functions require a solution word
///in which case plugging in whatever for that argument is fine.
fn bucket_and_update<T>(
    guess: WordId,
    solution: WordId,
    compare: T,
    left: &[WordId],
    used: &mut [bool],
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
            used[word] = true;
        }
    }
    bkt
}

///Creates a bucket for a given word given
///a bank of guesses and possible solutions.
///note I only let buckets be composed of possible solution words
///Note that not all compare functions require a solution word
///in which case plugging in whatever for that argument is fine.
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
///I'll call this smallest largest bucket an sm_bucket
///and the word which creates that bucket the sm_word
fn sm_word(left: &[WordId], gbank: WordBank, sbank: WordBank) -> WordId {
    let smbl = left
        .par_iter()
        .fold(
            || vec![0; gbank.len()],
            |mut acc, &sol| {
                if sol % 50 == 0 { println!("solution word: {}", sol); }
                //create all the buckets
                let mut used = vec![false; sbank.len()];
                let mut bkts = vec![];
                for &gsol in left {
                    bkts.push(bucket(
                        gsol,
                        sol,
                        solution_compare,
                        left,
                        sbank,
                        sbank,
                    ));
                }

                //check to see bucket size for a given solution
                //if so update smbl
                for (guess, w) in acc.iter_mut().enumerate() {
                    let guess_mark = solution_compare(guess, sol, gbank, sbank);
                    for bkt in bkts.iter() {
                        let bmark = solution_compare(guess, bkt[0], gbank, sbank);
                        if guess_mark == bmark {
                            *w = (*w).max(bkt.len());
                            break;
                        }
                    }
                }
                acc
            },
        )
        .reduce(
            || vec![0; gbank.len()],
            |acc, prt| {
                acc.into_iter()
                    .zip(prt.into_iter())
                    .map(|(a, b)| a.max(b))
                    .collect::<Vec<_>>()
            },
        );

    println!("smbl: {:?}", smbl);
    //min should always succeed as there should exist non-zero buckets
    smbl.iter()
        .position(|x| x == smbl.iter().filter(|&&w| w != 0).min().unwrap())
        .unwrap()
}

///Starting word, guess with minimum maxmimum bucket
///found with a brute force in about 2 and a half minutes
///the word is aggri
/////the word was "miaou"
//const START_GUESS_WORD: WordId = 5660;
const START_GUESS_WORD: WordId = 132;

///The game will let you define a way to guess and a way to get the current solution.
///It requires a way to determine a guess and a way to determine how that guess will be marked.
///It provides an update function which can be used to play through the game. It will return
///either Some(bucket) or None if the game is over
trait Game {
    ///Guesses a word with a certain word ID
    fn guess(&mut self) -> WordId;
    ///returns the current solution word, this can depend on the guess but should be consistant to
    ///the actual rules of a proper wordle game.
    fn solution(&mut self, guess: WordId) -> WordId;
    ///Updates the current game, returning a Bucket of remain words or None if the game has
    ///terminated. It will update the current game state.
    fn update(&mut self) -> Option<Bucket>;
}

///A game struct for the standard wordle game with one word held constant.
struct FixedWordle<'a> {
    left: Bucket,
    //this will be sbank
    solution: WordId,
    sbank: &'a [&'a str],
    gbank: &'a [&'a str],
    hard_coded_first_turn: bool,
}

impl<'a> FixedWordle<'a> {
    fn with_state(
        solution: WordId,
        left: Bucket,
        sbank: &'a [&str],
        gbank: &'a [&str],
        hard_coded_first_turn: bool,
    ) -> Self {
        Self {
            left,
            solution,
            sbank,
            gbank,
            hard_coded_first_turn,
        }
    }
}

impl<'a> Game for FixedWordle<'a> {
    //This guess can take a long time, but shouldn't if the
    fn guess(&mut self) -> WordId {
        if self.hard_coded_first_turn {
            self.hard_coded_first_turn = false;
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

//utility functions to do things so main is less a mass of comments
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

//sims a game using a greedy algo and the given solution word as the solution.
//returns the number of turns the game took.
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
        println!("one turn: {}", cur_turn);
        println!("cur_game: {:?}", cur_game_bkt);
        let bkt = cur_game_bkt.unwrap();
        assert!(bkt.contains(&solution));
        cur_game_bkt = game.update();
        cur_turn += 1;
    }
    cur_turn
}

//for every solution word, create all the buckets of solution words
//for every guess see which one it matches
//keep a table which maps: guess word -> maximum bucket size
fn main() {
    //get_smw();
    /*
    let guesses = GUESSES_STR.split('\n').map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>();
    let solutions = SOLUTIONS_STR.split('\n').map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>();
    let solution = 0;
    println!("game with solution \"{}\" takes {} turns", solution, sim_game_with_solution(solution, &guesses, &solutions));
    */
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
    println!("{}", guesses[START_GUESS_WORD]);
    let mut mxlen = 0;
    let mut mx = vec![];
    for i in 0..solutions.len() {
        let bkt = bucket(
            START_GUESS_WORD, 
            i,
            solution_compare,
            &(0..solutions.len()).collect::<Vec<_>>(), 
            &guesses, 
            &solutions,
        );
        if bkt.len() > mxlen {
            mxlen = bkt.len();
            mx = bkt;
        }
    }
    println!("{}", mxlen);
    println!("{:?}", mx);
}
