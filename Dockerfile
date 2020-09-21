FROM rustlang/rust:nightly
RUN cargo --version
ADD . /boluo
RUN cd /boluo && cargo build --release && cp /boluo/target/release/server /bin && cp /boluo/target/release/manage /bin && rm -rf /boluo
WORKDIR /
