FROM node:latest AS node_base

RUN echo "NODE Version:" && node --version
RUN echo "NPM Version:" && npm --version

FROM rustlang/rust:nightly as builder

COPY --from=node_base /usr/local/bin /usr/local/bin
COPY --from=node_base /usr/local/lib/node_modules/npm /usr/local/lib/node_modules/npm

WORKDIR /app
COPY . .

# Install sass
RUN npm install -g sass

# Download Chrome for scraping

RUN apt-get update
RUN apt-get install -y wget
RUN wget -q https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb
RUN apt-get install -y ./google-chrome-stable_current_amd64.deb

# update rust just in case
RUN rustup update

# Install cargo-leptos for building
RUN cargo install --locked cargo-leptos

# target wasm32-unknown-unknown 
RUN rustup target add wasm32-unknown-unknown 
RUN rustup target add x86_64-unknown-linux-gnu 

# set env variables for build
ENV LEPTOS_BIN_TARGET_TRIPLE x86_64-unknown-linux-gnu
ENV LEPTOS_ENV PROD
ENV LEPTOS_OUTPUT_NAME mr-modpack
ENV LEPTOS_HASH_FILES true

# RUN cargo update -p wasm-bindgen

# Build with cargo-leptos
RUN cargo leptos build --release -vv

# start the application
CMD ./target/x86_64-unknown-linux-gnu/release/mr-modpack
