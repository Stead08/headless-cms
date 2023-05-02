-- Add migration script here
CREATE TABLE IF NOT EXISTS users (
                                     id SERIAL PRIMARY KEY,
                                     username VARCHAR UNIQUE NOT NULL,
                                     email VARCHAR UNIQUE NOT NULL,
                                     password VARCHAR NOT NULL,
                                     createdAt TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS sessions (
                                        id SERIAL PRIMARY KEY,
                                        session_id VARCHAR NOT NULL UNIQUE,
                                        user_id INT NOT NULL UNIQUE
);