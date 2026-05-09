.PHONY: build test lint audit check adversarial check-dead-params check-test-count bench bench-pizza bench-ontoaxiom bench-mushroom bench-vision bench-reasoner bench-oaei docker docker-run init serve serve-http clean

# ─── Development ─────────────────────────────────────────────────────────────

build:
	cargo build --release

test:
	cargo test

lint:
	cargo clippy -- -D warnings

check-dead-params:
	bash tools/dead-param-gate.sh

check-test-count:
	bash tools/check-test-count.sh

adversarial: check-dead-params check-test-count
	cargo clippy -- -D clippy::todo -D clippy::unimplemented
	cargo test --test adversarial_jtbd_test -- --test-threads=1
	@echo "✓ All adversarial JTBD gates passed"

audit:
	cargo audit

check: check-dead-params lint test audit

# ─── Benchmarks ──────────────────────────────────────────────────────────────

bench: bench-pizza bench-ontoaxiom bench-mushroom bench-reasoner bench-oaei
	@echo "All benchmarks complete."

bench-pizza:
	@echo "=== Pizza Ontology Benchmark ==="
	cd benchmark && python3 pizza_benchmark.py 2>/dev/null || echo "Requires Python 3 + rdflib"

bench-ontoaxiom:
	@echo "=== OntoAxiom Benchmark ==="
	cd benchmark/ontoaxiom && python3 run_benchmark.py 2>/dev/null || echo "Requires Python 3 + dependencies"

bench-mushroom:
	@echo "=== Mushroom Classification Benchmark ==="
	cd benchmark/mushroom && python3 mushroom_benchmark.py 2>/dev/null || echo "Requires Python 3 + dependencies"

bench-vision:
	@echo "=== Vision Pipeline Benchmark ==="
	cd benchmark/vision && python3 vision_benchmark.py 2>/dev/null || echo "Requires Python 3 + dependencies"

bench-reasoner:
	@echo "=== Reasoner Comparison (HermiT vs Open Ontologies) ==="
	cd benchmark/reasoner && python3 compare_reasoners.py 2>/dev/null || echo "Requires Python 3 + Java for HermiT"

bench-oaei:
	@echo "=== OAEI Alignment Benchmark ==="
	cd benchmark/oaei && python3 download_oaei.py && python3 run_oaei_benchmark.py 2>/dev/null || echo "Requires Python 3 + mcp SDK"

# ─── Docker ──────────────────────────────────────────────────────────────────

docker:
	docker build -t open-ontologies:latest .

docker-run:
	docker run -i open-ontologies:latest serve

# ─── Release ─────────────────────────────────────────────────────────────────

init:
	cargo run --release -- init

serve:
	cargo run --release -- serve

serve-http:
	cargo run --release -- serve-http

# ─── Cleanup ─────────────────────────────────────────────────────────────────

clean:
	cargo clean
