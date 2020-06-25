FROM node:14 as client
WORKDIR /build

RUN npm -g config set user root
RUN npm install -g bower
RUN npm install -g polymer-cli

COPY ./client/package.json ./client/package-lock.json ./
RUN npm install

COPY ./client/tsconfig.json ./client/index.html ./
COPY ./client/src ./src

RUN npm run build

FROM rustlang/rust:nightly AS server
WORKDIR /build

# Download the target for static linking.
RUN rustup target add x86_64-unknown-linux-gnu

RUN USER=root cargo new tweet_analyzer_server
WORKDIR /build/tweet_analyzer_server

COPY ./server/Cargo.toml ./server/Cargo.lock ./
RUN cargo build --release

# Copy the source and build the application.
COPY ./server/src ./src
RUN cargo build --release --target x86_64-unknown-linux-gnu

# Copy the statically-linked binary into a scratch container.
FROM ubuntu
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=server /build/tweet_analyzer_server/target/x86_64-unknown-linux-gnu/release/tweet_analyzer_server .
COPY --from=client /build/build/default ./static
COPY entrypoint.sh .
RUN chmod a+x entrypoint.sh

ENTRYPOINT [ "/entrypoint.sh" ]