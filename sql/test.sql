DROP TABLE fullfillment;
DROP TABLE requirement;
DROP TABLE participant;
DROP TABLE event;
DROP TABLE user;

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

INSERT INTO user (id, username, password, salt) VALUES (1, 'username1', 'd332ef80281d79e3dd4c5f0ea7e782fc689842e70cf606b84e8bb9fb676626e', 'kE(mL@^0');
INSERT INTO user (id, username, password, salt) VALUES (2, 'username2', '11c3538c6236c9e61554df8c32662206c4220e8aa3da8dfbb384fea880e', 'OO4tO7pB');
INSERT INTO user (id, username, password, salt) VALUES (3, 'username3', 'db6a21ccba5441fad1eecb44a8f3ff73f16dd4ba3c57aa1ef8cc168642ec0e1', 'E)Qpt2ry');
INSERT INTO user (id, username, password, salt) VALUES (4, 'username4', '968e608577eedcfcc4a7f1418b2a76c12884c305d5b6b18269827ad25d7299', 'gL9m51s4');
INSERT INTO user (id, username, password, salt) VALUES (5, 'username5', 'dee1bc54771caa7cfce48eac3f8c3e781781864c7c63fbcca077feb7bca6e4c', 'qADT#zEn');
INSERT INTO user (id, username, password, salt) VALUES (6, 'username6', 'd5e6a29c675d797d57a2644a2f2ae223c8744ae11c5233ce417426fbb1158', '!hk)fsQu');
INSERT INTO event (id, name, description, creator) VALUES (1, 'event-1', 'some description 1', 1);
INSERT INTO event (id, name, description, creator) VALUES (2, 'event-2', 'some description 2', 6);
INSERT INTO event (id, name, description, creator) VALUES (3, 'event-3', 'some description 3', 4);
INSERT INTO participant (user, event) VALUES (2, 1);
INSERT INTO participant (user, event) VALUES (3, 1);
INSERT INTO participant (user, event) VALUES (3, 2);
INSERT INTO participant (user, event) VALUES (4, 2);
INSERT INTO requirement (id, name, description, event, size) VALUES (1, "req1", "req1-desc", 1, 2);
INSERT INTO requirement (id, name, description, event, size) VALUES (2, "req2", "req2-desc", 1, 1);
INSERT INTO requirement (id, name, description, event, size) VALUES (3, "req3", "req3-desc", 2, 1);
INSERT INTO fullfillment (user, requirement) VALUES (4, 1);
INSERT INTO fullfillment (user, requirement) VALUES (2, 3);
