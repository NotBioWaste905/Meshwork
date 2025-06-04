# Use Python 3.12 to match .python-version file
# Use slim-bullseye instead of alpine for better compatibility with scientific packages
FROM python:3.12-slim-bullseye as builder

# Install system dependencies needed for building Python packages
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install poetry (isolated installation)
ENV POETRY_HOME=/opt/poetry
ENV POETRY_VERSION=2.1.0
RUN curl -sSL https://install.python-poetry.org | python3 - && \
    cd /usr/local/bin && \
    ln -s /opt/poetry/bin/poetry && \
    poetry config virtualenvs.in-project true

# Set working directory
WORKDIR /app

# Copy dependency files
COPY pyproject.toml poetry.lock ./

# Install dependencies in a virtual environment
RUN poetry install --no-interaction --no-ansi --only main --no-root

# Production stage
FROM python:3.12-slim-bullseye

# Install runtime dependencies for scientific packages
RUN apt-get update && apt-get install -y \
    libgomp1 \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user for security
RUN groupadd -r appuser && useradd -r -g appuser appuser

# Set working directory
WORKDIR /app

# Copy virtual environment from builder stage
COPY --from=builder /app/.venv /app/.venv

# Copy application code
COPY . .

# Change ownership of app directory to appuser
RUN chown -R appuser:appuser /app

# Switch to non-root user
USER appuser

# Add virtual environment to PATH
ENV PATH="/app/.venv/bin:$PATH"

# Set Python path
ENV PYTHONPATH=/app

# Health check using curl instead of requests
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

# Expose port
EXPOSE 8000

# Use proper signal handling and run on port 8000 (standard for development)
CMD ["uvicorn", "app:app", "--host", "0.0.0.0", "--port", "8000"]
