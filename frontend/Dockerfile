# Build stage
FROM docker.io/denoland/deno:latest AS build
WORKDIR /app
COPY package*.json deno.lock ./
COPY . .

ARG WS_BASE_URL=/ws
ENV PUBLIC_WS_BASE_URL=$WS_BASE_URL
RUN deno task build # sveltekit build using adapter-static -> output in build

# Runtime stage
FROM docker.io/nginx:stable-alpine-slim AS runtime
# copy built static files
COPY --from=build /app/build /usr/share/nginx/html

# copy nginx conf
COPY nginx.conf /etc/nginx/nginx.conf

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
