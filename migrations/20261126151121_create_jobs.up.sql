create table jobs (
    id bigserial primary key,
    queue text not null,
    name text not null,
    payload text not null,
    attempts integer not null default 0,
    max_attempts integer not null,
    available_at bigint not null,
    reserved_until bigint,
    last_error text,
    created_at bigint not null
);

create index jobs_claim on jobs (queue, available_at);

create table failed_jobs (
    id bigint primary key,
    queue text not null,
    name text not null,
    payload text not null,
    attempts integer not null,
    max_attempts integer not null,
    error text not null,
    failed_at bigint not null
);
