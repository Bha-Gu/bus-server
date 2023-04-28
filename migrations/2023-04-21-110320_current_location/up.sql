-- Your SQL goes here
CREATE TABLE current_location (
    busid CHAR(12) NOT NULL PRIMARY KEY,
    latitude FLOAT NOT NULL,
    longitude FLOAT NOT NULL
)