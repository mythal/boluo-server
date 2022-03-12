FROM rustlang/rust:nightly-buster AS builder
RUN cargo --version
ADD . /boluo
RUN cd /boluo && cargo build --release

FROM debian:buster AS server
COPY --from=builder /boluo/target/release/server /bin/server
COPY --from=builder /boluo/target/release/manage /bin/manage
