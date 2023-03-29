package com.lichess.puzzle;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.FileReader;
import java.io.FileWriter;
import java.io.IOException;

public class csvfilter {
    public void filterCsvFile(String inputFilename, String outputFilename, int nums) throws IOException {
        try (BufferedReader reader = new BufferedReader(new FileReader(inputFilename));
                BufferedWriter writer = new BufferedWriter(new FileWriter(outputFilename))) {

            String line;
            while ((line = reader.readLine()) != null) {
                String[] fields = line.split(",");
                if (fields.length < 9) {
                    continue;
                }
                String puzzleId = fields[0];
                String fen = fields[1];
                String moves = fields[2];
                String rating = fields[3];
                String ratingDeviation = fields[4];
                String popularity = fields[5];
                String nbPlays = fields[6];
                String themes = fields[7];
                String gameUrl = fields[8];
                // String openingTags = fields[9] == null ? "" : fields[9];

                int ratingInt = Integer.parseInt(rating);

                if (ratingInt > nums) {
                    writer.write(String.format("[Event \"%s\"]%n", puzzleId));
                    writer.write(String.format("[FEN \"%s\"]%n", fen));
                    writer.write(String.format("[Site \"%s\"]%n", gameUrl));
                    writer.write(String.format("[White \"%s\"]%n", rating));
                    writer.write(String.format("[WhiteElo \"%s\"]%n", ratingDeviation));
                    writer.write(String.format("[Popularity \"%s\"]%n", popularity));
                    writer.write(String.format("[BlackElo \"%s\"]%n", nbPlays));
                    writer.write(String.format("[Black \"%s\"]%n", themes));
                    // writer.write(String.format("[Annotator \"%s\"]%n", openingTags));
                    writer.write(String.format("%s 1-0%n%n", moves));
                }

            }
        } catch (IOException e) {
            System.err.format("IOException: %s%n", e);
        }
    }
}
