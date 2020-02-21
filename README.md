# [Boluo](https://github.com/mythal/boluo) Server

A chat tool made for play RPG.

## Set Up

### Docker

```bash
cp .env.docker.template .env
docker-compose build
docker-compose up

# test
docker-compose run server cargo test
```

### Non-Docker

First, set up Redis and Postgres database, then execute `schema.sql` on the database.

```bash
cp .env.dev.template .env # edit it
export $(cat .env | xargs)
cargo test
```
