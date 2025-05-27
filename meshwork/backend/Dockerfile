FROM python:3.13.3-slim-bookworm

WORKDIR /app

# Install dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy application code
COPY . .

EXPOSE 8000

# Option 1: Run with Uvicorn directly
CMD ["uvicorn", "app:app", "--host", "0.0.0.0", "--port", "8000"]
