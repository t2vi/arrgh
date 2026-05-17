FROM node:20-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci --omit=dev
COPY . .
RUN npm run build
EXPOSE 4003
ENV PORT=4003 API_KEY=
CMD ["node", "dist/index.js"]
