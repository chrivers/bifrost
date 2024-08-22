FROM rust:latest

WORKDIR /app/bifrost

COPY . .

RUN cargo build

CMD ["cargo", "run"]