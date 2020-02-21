FROM rustlang/rust:nightly
RUN cargo --version

RUN mkdir /boluo
WORKDIR /boluo
ADD . /boluo
RUN cargo build --release
