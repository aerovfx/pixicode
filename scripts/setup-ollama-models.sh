#!/usr/bin/env bash
# Load Ollama models for speculative decoding tests:
#   - Draft (nhỏ): qwen3.5:0.8b; hoặc qwen2:0.5b / qwen3:0.6b.
#   - Target: edu-assistant:latest (thường có sẵn local) hoặc qwen3-coder:480b-cloud, v.v.
# Requires: Ollama installed and running (ollama serve).

set -e

if ! command -v ollama &>/dev/null; then
  echo "Ollama not found. Install from https://ollama.com"
  exit 1
fi

echo "Pulling draft model qwen3.5:0.8b..."
if ollama pull qwen3.5:0.8b 2>/dev/null; then
  echo "qwen3.5:0.8b OK."
else
  echo "qwen3.5:0.8b không pull được. Thử: ollama pull qwen3:0.6b hoặc qwen2:0.5b; đổi config speculative.draft tương ứng."
fi

echo "Target edu-assistant:latest (thường đã có local)..."
if ollama pull edu-assistant:latest 2>/dev/null; then
  echo "edu-assistant:latest pulled."
else
  echo "edu-assistant:latest không trên Library — bạn đã có local (edu-assistant:latest, qwen3-coder:480b-cloud) thì bỏ qua."
fi

echo "Listing loaded models (digest in second column):"
ollama list

echo "Done. Draft mặc định: qwen3.5:0.8b. Xem docs/ollama-models-and-test.md."
