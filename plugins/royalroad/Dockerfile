FROM node:20-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci --omit=dev
COPY . .
RUN npm run build
EXPOSE 4004
ENV PORT=4004 API_KEY=
CMD ["node", "dist/index.js"]
