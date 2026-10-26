# PLACEHOLDER: deploy target not decided yet. Builds any app in apps/ once they exist:
#   docker build --build-arg APP=apis -t apis .
ARG APP=apis

FROM rust:1.97-slim AS build
ARG APP
WORKDIR /src
COPY . .
RUN cargo build --release -p ${APP}

FROM debian:bookworm-slim
ARG APP
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/${APP} /usr/local/bin/app
USER 65534
ENTRYPOINT ["/usr/local/bin/app"]
