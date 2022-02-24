# wordle
The program simulates a greedy strategy on all the possible wordle games to figure the number of turns it requires in the worst case.
It also can compute the "best" starting word using a greedily choosing smallest buckets and looking one turn deep.
It might also do some other stuff in the future if I want it to. 
## To Run
```sh
cargo run --release
```
This should print a help message which will give the information needed. If running with `cargo run --release` as above, replace instances of `wordle` with `cargo run --release`. For example, `cargo run --release max-game`.
