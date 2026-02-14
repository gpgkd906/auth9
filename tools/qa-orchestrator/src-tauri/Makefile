COVERAGE_MIN ?= 90

.PHONY: test coverage coverage-html

test:
	cargo test --lib --bins

coverage:
	cargo llvm-cov --lib --fail-under-lines $(COVERAGE_MIN) --ignore-filename-regex 'build.rs'

coverage-html:
	cargo llvm-cov --lib --html --fail-under-lines $(COVERAGE_MIN) --ignore-filename-regex 'build.rs'
