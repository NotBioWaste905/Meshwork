# Node 24 is the current Active LTS line.
FROM node:24-slim AS build

# vite and svelte-kit are devDependencies, so no NODE_ENV=production here

WORKDIR /app

COPY package.json package-lock.json .npmrc ./
RUN npm ci
COPY . ./
RUN npm run build


FROM node:24-slim

WORKDIR /app
# `vite preview` serves the build and needs vite and the svelte config at runtime;
# it also writes the bundled config into node_modules/.vite-temp, hence --chown
COPY --from=build --chown=node:node /app .

ENV HOST=0.0.0.0
EXPOSE 3000
USER node
CMD ["npm", "run", "preview", "--", "--port", "3000", "--host", "0.0.0.0"]
