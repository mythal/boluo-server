# [Boluo](https://github.com/mythal/boluo) Server

A chat tool made for play RPG.

## Setup

```
cp .env.template .env
cargo install diesel_cli --no-default-features --features postgres
diesel migration run
cargo test
```
