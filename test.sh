#!/bin/bash
set -e
dropdb --if-exists boluo_test
createdb boluo_test
psql boluo_test < schema.sql
export TEST_DATABASE_URL="postgresql://${USER}@localhost/boluo_test"
export "$(xargs < .env)"
cargo test
