CREATE TABLE chats (
    id BIGINT PRIMARY KEY,
    title VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

SELECT diesel_manage_updated_at('chats');
