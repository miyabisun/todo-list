# syntax=docker/dockerfile:1

FROM node:24-bookworm-slim AS frontend
WORKDIR /app/client
COPY client/package.json client/package-lock.json ./
RUN npm ci
COPY client/ ./
RUN npm run build

# cargo-chef builds dependencies from a recipe that masks the package version,
# so source edits and release bumps reuse the compiled dependency layer.
FROM rust:1.96-bookworm AS chef
WORKDIR /app
COPY rust-toolchain.toml ./
RUN cargo install cargo-chef --version 0.1.78 --locked

FROM chef AS planner
COPY Cargo.toml Cargo.lock build.rs ./
COPY src/ src/
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS backend
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --locked --release --recipe-path recipe.json
COPY Cargo.toml Cargo.lock build.rs ./
COPY src/ src/
COPY --from=frontend /app/client/dist ./client/dist
RUN cargo build --locked --release

FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=backend /app/target/release/todo-list /usr/local/bin/todo-list
# One Markdown file per to-do below APP_DATA_DIR; the runtime user owns /app/data.
RUN install -d -o 10001 -g 10001 /app/data
VOLUME ["/app/data"]
ENV PORT=3000
ENV APP_DATA_DIR=/app/data/todos
EXPOSE 3000
USER 10001:10001
ENTRYPOINT ["todo-list"]
