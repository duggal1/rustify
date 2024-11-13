FROM oven/bun
WORKDIR /app
COPY package*.json ./
RUN bun install
COPY . .
ENV PORT=3000
EXPOSE 3000
CMD npm run dev