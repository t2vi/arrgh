FROM node:22-alpine AS build
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY tsconfig.json .
COPY src ./src
RUN npm run build

FROM node:22-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci --omit=dev
COPY --from=build /app/dist ./dist
ENV PORT=4002 API_KEY= FLARESOLVERR_URL=http://flaresolverr:8191
EXPOSE 4002
CMD ["node", "dist/index.js"]
