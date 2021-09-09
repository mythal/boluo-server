#!/bin/bash
dropdb boluo_test
createdb boluo_test
psql boluo_test < schema.sql
export TEST_DATABASE_URL="postgresql://${USER}@localhost/boluo_test"
if [ ! -f .env ]
then
  export "$(xargs < .env)"
fi
cargo test
