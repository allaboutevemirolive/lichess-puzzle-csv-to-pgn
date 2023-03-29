package com.lichess.puzzle;

import java.io.File;
import java.io.IOException;
import java.util.concurrent.TimeUnit;
import javafx.application.Application;
import javafx.application.Platform;
import javafx.geometry.Insets;
import javafx.geometry.Pos;
import javafx.scene.Scene;
import javafx.scene.control.Button;
import javafx.scene.control.Hyperlink;
import javafx.scene.control.Label;
import javafx.scene.control.TextField;
import javafx.scene.layout.BorderPane;
import javafx.scene.layout.HBox;
import javafx.scene.layout.VBox;
import javafx.scene.paint.Color;
import javafx.stage.FileChooser;
import javafx.stage.Stage;

public class Main extends Application {

    private final csvfilter obj = new csvfilter();

    // Status label for showing messages
    private final Label statusLabel = new Label();

    private static final String FILE_SELECTED_MESSAGE = "File selected: ";

    private TextField numsField;

    @Override
    public void start(Stage primaryStage) {

        primaryStage.setTitle("Lichess Puzzle CSV to PGN"); // Set the title
        // primaryStage.getIcons().add(new Image("C:/Users/akmal/OneDrive/Desktop/LichessPuzzle/lichess/Icon/Rook.png")); // Set the icon

        // Root pane of BorderPane
        BorderPane root = new BorderPane();
        root.setPadding(new Insets(10));

        numsField = new TextField();
        numsField.setPromptText("Enter minimum rating");

        // Select Button for choosing CSV file from File Chooser
        Button selectButton = new Button("Select CSV File");
        selectButton.setOnAction(e -> {
            // Choose CSV file from file chooser and set the selected file name to status
            // label
            FileChooser fileChooser = new FileChooser();
            File selectedFile = fileChooser.showOpenDialog(primaryStage);
            if (selectedFile != null) {
                statusLabel.setText(FILE_SELECTED_MESSAGE + selectedFile.getAbsolutePath());
            }
        });

        // Convert Button for converting CSV file into PGN format
        Button convertButton = new Button("Convert File");
        convertButton.setOnAction(e -> {
            // replace() method could potentially cause issues if the text in the statusLabel changes in the future
            // It's better to check if the text contains the substring "File selected: " instead.
            String selectedFile = statusLabel.getText();
            if (selectedFile.contains(FILE_SELECTED_MESSAGE)) {
                selectedFile = selectedFile.replace(FILE_SELECTED_MESSAGE, "");
                convertCsvToPgn(selectedFile);
            } else {
                statusLabel.setText("Please select a file first.");
            }
        });        

        // Info button for displaying author information
        Button infoButton = new Button("Info");
        infoButton.setOnAction(e -> 
            showInfoPopup()
        );

        // Set width and alignment of status label
        statusLabel.setPrefWidth(400);
        statusLabel.setAlignment(Pos.CENTER);

        // Create a VBox to store "Minimum rating" label and numsField TextField
        VBox vBox = new VBox(10, new Label("Minimum rating:"), numsField);

        // Create an HBox to store buttons
        HBox buttonsBox = new HBox(10, selectButton, convertButton, infoButton);
        buttonsBox.setAlignment(Pos.CENTER);

        // Create a VBox to store topBox and vBox
        VBox mainBox = new VBox(10, vBox, buttonsBox);

        // Create a StackPane to display message/status and watermark
        VBox bottomPane = new VBox();
        bottomPane.setAlignment(Pos.BOTTOM_RIGHT);
        bottomPane.getChildren().addAll(statusLabel, createWatermarkLabel());

        // Add mainBox to top and status pane to bottom of BorderPane
        root.setTop(mainBox);
        root.setBottom(bottomPane);

        Scene scene = new Scene(root, 350, 150);
        primaryStage.setScene(scene);
        primaryStage.show();
    }

    private Label createWatermarkLabel() {
        Label watermarkLabel = new Label("2023");
        watermarkLabel.setTextFill(Color.GRAY);
        watermarkLabel.setOpacity(0.5);
        return watermarkLabel;
    }

    // Function for converting CSV file into PGN format
    private void convertCsvToPgn(String inputFilename) {
        // Output filename with .pgn extension
        String outputFilename = "output.pgn";

        // Run conversion process in thread
        new Thread(() -> {
            long startTime = System.nanoTime();

            try {
                if (numsField.getText().isEmpty()) {
                    numsField.setText("0");
                }
                int nums = Integer.parseInt(numsField.getText());
                // Call the function to convert CSV file into PGN format
                obj.filterCsvFile(inputFilename, outputFilename, nums);
                // Show success message after execution
                Platform.runLater(() -> {
                    long executionTime = TimeUnit.NANOSECONDS.toMillis(System.nanoTime() - startTime);
                    statusLabel.setText("Conversion successful! Execution time: " + executionTime + "ms");
                });
            } catch (IOException ex) {
                // If error occurs during conversion, show error message
                ex.printStackTrace();
                Platform.runLater(() -> 
                    statusLabel.setText("Error: " + ex.getMessage())
                );
            }
        }).start();
    }

    // Function for displaying information popup about the author
    private void showInfoPopup() {
        // Information labels
        Label authorLabel = new Label("Author: allaboutevemirolive");
        Label versionLabel = new Label("Version: 1.1");
        Label rightsLabel = new Label("Like this project? Consider donating to the author via Wise or PayPal.");
        Label payPalEmail = new Label("PayPal: xkmxlfirdxus@gmail.com");
        Label wiseEmail = new Label("Wise: pressdoodle@gmail.com");

        Hyperlink databaseLink = new Hyperlink("Lichess Puzzle Database");
        databaseLink.setOnAction(e -> 
            getHostServices().showDocument("https://database.lichess.org/#puzzles")
        );

        // Hyperlinks to author's social media accounts
        Hyperlink twitterLink = new Hyperlink("Twitter");
        twitterLink.setOnAction(e -> 
            getHostServices().showDocument("https://twitter.com/akmalfirdxus")
        );

        Hyperlink githubLink = new Hyperlink("GitHub");
        githubLink.setOnAction(e -> 
            getHostServices().showDocument("https://github.com/allaboutevemirolive")
        );

        Hyperlink instagramLink = new Hyperlink("Instagram");
        instagramLink.setOnAction(e -> 
            getHostServices().showDocument("https://www.instagram.com/akmxlfirdaus/")
        );

        HBox linksBox = new HBox(10, databaseLink, twitterLink, githubLink, instagramLink);
        linksBox.setAlignment(Pos.CENTER);

        VBox infoBox = new VBox(10, authorLabel, versionLabel, rightsLabel, wiseEmail, payPalEmail, linksBox);
        infoBox.setAlignment(Pos.CENTER);
        infoBox.setPadding(new Insets(10));

        Scene infoScene = new Scene(infoBox, 500, 200);

        Stage infoStage = new Stage();
        infoStage.setScene(infoScene);
        infoStage.setTitle("Info");
        infoStage.show();

    }

    public static void main(String[] args) {
        launch(args);
    }
}
