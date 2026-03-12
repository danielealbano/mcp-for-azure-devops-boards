FROM ubuntu:22.04
RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN curl https://cursor.com/install -fsSL | bash
ENV PATH="/root/.local/bin:${PATH}"
