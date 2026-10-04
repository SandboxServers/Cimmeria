FROM python:3.13-slim
RUN useradd --system --uid 10091 --no-create-home broker
COPY broker.py /app/broker.py
USER broker
ENV PYTHONUNBUFFERED=1
ENTRYPOINT ["python", "/app/broker.py"]
