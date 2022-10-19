FROM rust:1 as builder
WORKDIR /app
COPY . .
RUN cargo install --path .

FROM debian:buster-slim as runner
WORKDIR /app
COPY --from=builder /usr/local/cargo/bin/leaderboard /app/leaderboard
ENV ROCKET_ADDRESS=0.0.0.0
EXPOSE 8000
CMD ["./leaderboard"]