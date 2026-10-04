FROM python:3.11-slim
RUN useradd --system --uid 10092 --no-create-home mailer
COPY mailer.py /app/mailer.py
USER mailer
ENV PYTHONUNBUFFERED=1 PYTHONWARNINGS=ignore::DeprecationWarning
ENTRYPOINT ["python", "/app/mailer.py"]
