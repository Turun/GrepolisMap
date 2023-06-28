# Goals:

This program should offer a way to view the map of a Server and show the cities on it. 
The user should be able to define the color of a group. A group is the set of all player following user defined restrictions. 
These restriction can be by playername
- alliance name
- city location
- player location 
    has at least one city within range xyz
    has no city within range xyz


# Software

I want to write this in rust. The main concern is to get a GUI Framework that allows us to easily draw the map. I have experience with egui, but iced looks nice too.
Both iced and egui have canvas elements (see examples iced::clock, iced::colorpalette, egui::painter_demo), onto which we can draw arbitrary shapes. I have used egui before, but I also want to try out iced. Maybe I'll go with iced to try that framework out as well.
EDIT: Update 05-29: iced canvas is broken. Will have to use egui. I'll try my best to keep to the mvp pattern anyway, but I know it'll be difficult. Last time I checked egui didn't lend itself easily to that pattern.

I like the model-view-presenter pattern which provides a hard and clear boundary between the front- and backend. I'll likely use this pattern here as well. We'll see how well that integrates into the GUI framework though (leaning towards iced at the moment.)

# Layout

On the left we will have a sidepanel where the user can set which cities are displayed in which color.
The center will have the map.
Optionally we can use the bottom or right side to display stats. Which stats that would be I do not know yet, but if we wan't to that's where they would probably go.

# Timeseries

This is not the core functionality, but timeseries data would be epic. 

We would have to run the program continuously on a server, which requests all data from Grepolis regularly, and save a copy whenever the data changes. It would probably best to do this in a database and only save the differences between updates. This would hopefully bring big improvements in disk space, at the cost of processing time for each request. Once a day or once a week we could save the complete copy, thereby reducing the processing time. 
It remains to be seen what the best format for saving the differences will be.



