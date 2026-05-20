FROM cloakhq/cloakbrowser
RUN apt-get update && apt-get install -y nginx && rm -rf /var/lib/apt/lists/*
# Pre-download the stealth Chromium binary at build time
RUN python -m cloakbrowser install
COPY serve.py /app/serve.py
COPY nginx.conf /etc/nginx/nginx.conf
ENV CDP_PORT=3000
EXPOSE 3000
ENTRYPOINT ["python", "/app/serve.py"]
