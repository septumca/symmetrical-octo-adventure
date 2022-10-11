CREATE TABLE IF NOT EXISTS user (
    id INTEGER PRIMARY KEY,
    username TEXT NOT NULL,
    password TEXT NOT NULL,
    salt TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS event (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    creator INTEGER NOT NULL,
    FOREIGN KEY(creator) REFERENCES user(id)
);

CREATE TABLE IF NOT EXISTS participant (
    event INTEGER NOT NULL,
    user INTEGER NOT NULL,
    FOREIGN KEY(event) REFERENCES event(id),
    FOREIGN KEY(user) REFERENCES user(id)
);

CREATE TABLE IF NOT EXISTS requirement (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    event INTEGER NOT NULL,
    FOREIGN KEY(event) REFERENCES event(id)
);

CREATE TABLE IF NOT EXISTS fullfillment (
    id INTEGER PRIMARY KEY,
    user INTEGER NOT NULL,
    requirement INTEGER NOT NULL,
    FOREIGN KEY(user) REFERENCES user(id),
    FOREIGN KEY(requirement) REFERENCES requirement(id)
);
