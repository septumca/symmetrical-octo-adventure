CREATE TABLE IF NOT EXISTS user (
    id INTEGER PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
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
    user INTEGER NOT NULL,
    event INTEGER NOT NULL,
    PRIMARY KEY(user, event),
    FOREIGN KEY(event) REFERENCES event(id),
    FOREIGN KEY(user) REFERENCES user(id)
);

CREATE TABLE IF NOT EXISTS requirement (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    event INTEGER NOT NULL,
    size INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY(event) REFERENCES event(id)
);

CREATE TABLE IF NOT EXISTS fullfillment (
    user INTEGER NOT NULL,
    requirement INTEGER NOT NULL,
    PRIMARY KEY(user, requirement),
    FOREIGN KEY(user) REFERENCES user(id),
    FOREIGN KEY(requirement) REFERENCES requirement(id)
);
