# Local LLM Adversarial Harness Setup
**SAB 900: The Saboteur Labs**

To conduct prompt injection and LLM hallucination attacks (Case Study 2) at scale without bankrupting the program via commercial API fees, students must set up a local, air-gapped LLM environment.

## 1. Local Engine Setup (Ollama)
We utilize `ollama` to run lightweight, high-speed models locally on Apple Silicon (M-series) or CUDA-enabled Linux workstations.

```bash
# Install Ollama
curl -fsSL https://ollama.com/install.sh | sh

# Pull a fast, highly-malleable model for adversarial testing
ollama run llama3.2:1b
ollama run phi3:mini
```

## 2. Configuring `open-ontologies` for Local Adversarial Execution
The `open-ontologies` configuration (`src/config.rs`) must be overridden to point `onto_translate_candidate` to the local adversarial node.

Create a `saboteur-config.toml`:
```toml
[llm]
engine = "openai" # We use the OpenAI-compatible endpoint provided by Ollama
api_base = "http://127.0.0.1:11434/v1"
model = "llama3.2:1b"
api_key_env = "DUMMY_KEY"
```

## 3. Execution Pipeline
Students will write bash loops or Python scripts to brute-force the LLM boundary:

```bash
#!/bin/bash
# Brute-force the LlmInput sanitizer with known LLM jailbreak sequences
for prompt in $(cat jailbreaks.txt); do
  export GROQ_API_KEY="DUMMY"
  open-ontologies server translate_candidate_ctq_full \
    --prompt "$prompt" \
    --config saboteur-config.toml >> saboteur_logs.jsonl
done
```

*Objective:* Generate a payload that causes the local LLM to output a valid JSON route bypassing the semantic constraints, proving the necessity of the downstream DL Reasoner (L5) and Cryptographic Gates (L4).