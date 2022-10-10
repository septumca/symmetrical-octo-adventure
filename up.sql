CREATE TABLE user (
    id INTEGER PRIMARY KEY,
    username TEXT NOT NULL,
    password TEXT NOT NULL,
    salt TEXT NOT NULL
);

CREATE TABLE event (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    creator INTEGER NOT NULL,
    FOREIGN KEY(creator) REFERENCES user(id)
);

CREATE TABLE participant (
    event INTEGER NOT NULL,
    user INTEGER NOT NULL,
    FOREIGN KEY(event) REFERENCES event(id),
    FOREIGN KEY(user) REFERENCES user(id)
);

CREATE TABLE requirement (
    id INTEGER PRIMARY KEY,
    descirption TEXT NOT NULL,
    size INTEGER NOT NULL,
    event INTEGER NOT NULL,
    FOREIGN KEY(event) REFERENCES event(id)
);

CREATE TABLE fullfillment (
    id INTEGER PRIMARY KEY,
    user INTEGER NOT NULL,
    requirement INTEGER NOT NULL,
    FOREIGN KEY(user) REFERENCES user(id),
    FOREIGN KEY(requirement) REFERENCES requirement(id)
);
