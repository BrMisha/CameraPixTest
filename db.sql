CREATE TABLE "main" (
                        "index"	INTEGER NOT NULL UNIQUE,
                        "time_p"	INTEGER NOT NULL,
                        "pitch"	REAL,
                        "time_i"	INTEGER,
                        "img"	BLOB,
                        PRIMARY KEY("index" AUTOINCREMENT)
);