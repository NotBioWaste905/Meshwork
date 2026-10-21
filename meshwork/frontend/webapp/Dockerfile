FROM node:19 AS build

# vite and svelte-kit are devDependencies, so no NODE_ENV=production here

WORKDIR /app

COPY package.json ./
COPY package-lock.json ./
RUN npm ci
COPY . ./
RUN npm run build


FROM node:19-slim

WORKDIR /app
COPY --from=build /app .


ENV HOST=0.0.0.0
EXPOSE 3000
CMD ["npm","run", "preview","--", "--port", "3000", "--host", "0.0.0.0"]
