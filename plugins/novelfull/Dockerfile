FROM node:20-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci --omit=dev
COPY . .
RUN npm run build
EXPOSE 4005
ENV PORT=4005 API_KEY= FLARESOLVERR_URL=http://flaresolverr:8191
CMD ["node", "dist/index.js"]
