# [Boluo](https://github.com/mythal/boluo) Server

A chat tool made for play RPG.

## Set Up

First, set up Redis and Postgres database, then execute `schema.sql` on the database.

```bash
createdb boluo
psql -U postgres boluo < schema.sql
cp .env.dev.template .env # edit it
export $(cat .env | xargs) && cargo test --release
```
