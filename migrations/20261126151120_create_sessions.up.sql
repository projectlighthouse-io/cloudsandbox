create table sessions (
    id text primary key,
    data bytea not null,
    expires_at bigint not null
);

create index sessions_expires_at on sessions (expires_at);
