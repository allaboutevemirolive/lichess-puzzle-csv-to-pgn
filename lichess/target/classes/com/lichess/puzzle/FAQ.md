```
I've noticed a problem (or a new version) in the Lichess Puzzle Database's fields. I can import my previous database but not the new one.


Is this a fresh version of LPD's fields?


Example :

1/1/2023 :

000hf,
r1bqk2r/pp1nbNp1/2p1p2p/8/2BP4/1PN3P1/P3QP1P/3R1RK1 b kq - 0 19,
e8f7 e2e6 f7f8 e6f7,
1574,
76,
88,
456,
mate mateIn2 middlegame short,
https://lichess.org/71ygsFeE/black#38,
Horwitz_Defense,
Horwitz_Defense_Other_variations (Notice this line)


3/28/2023 :

000hf,
r1bqk2r/pp1nbNp1/2p1p2p/8/2BP4/1PN3P1/P3QP1P/3R1RK1 b kq - 0 19,
e8f7 e2e6 f7f8 e6f7,
1554,
75,
89,
465,
mate mateIn2 middlegame short,
https://lichess.org/71ygsFeE/black#38,
Horwitz_Defense Horwitz_Defense_Other_variations (Notice this line)
```

https://database.lichess.org/#puzzles

To fix this problem please check Lichess Puzzle Database's fields in the link above. 

In the csv2pgn class, we can edit this line according to the number of fields in the database.

```
if (fields.length < 10) {
        continue;
}
```

Next, the following lines should be modified to tally with the number of fields in the database.

```
String puzzleId = fields[0];
String fen = fields[1];
String moves = fields[2];
String rating = fields[3];
String ratingDeviation = fields[4];
String popularity = fields[5];
String nbPlays = fields[6];
String themes = fields[7];
String gameUrl = fields[8];
String openingTags = fields[9];

writer.write(String.format("[Event \"%s\"]%n", puzzleId));
writer.write(String.format("[FEN \"%s\"]%n", fen));
writer.write(String.format("[Site \"%s\"]%n", gameUrl));
writer.write(String.format("[White \"%s\"]%n", rating));
writer.write(String.format("[WhiteElo \"%s\"]%n", ratingDeviation));
writer.write(String.format("[Popularity \"%s\"]%n", popularity));
writer.write(String.format("[BlackElo \"%s\"]%n", nbPlays));
writer.write(String.format("[Black \"%s\"]%n", themes));
writer.write(String.format("[Annotator \"%s\"]%n", openingTags));
writer.write(String.format("%s 1-0%n%n", moves));
```