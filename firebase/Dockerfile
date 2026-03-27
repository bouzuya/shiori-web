FROM node:24-trixie
RUN apt-get update && apt-get install -y --no-install-recommends \
    openjdk-21-jre-headless \
    && rm -rf /var/lib/apt/lists/*
RUN npm install -g firebase-tools && \
    firebase setup:emulators:firestore
WORKDIR /firebase
CMD ["firebase", "emulators:start", "--only", "firestore", "--project", "demo-project"]
