.PHONY: coverage coverage-html coverage-json

# Files excluded from coverage tracking:
# - repository/*.rs: Thin data mapping layer (ORM-equivalent), no business logic
# - main.rs: Program entry point
# - migration/*.rs: Database migration scripts
COVERAGE_EXCLUDE := --ignore-filename-regex 'repository/.*\.rs$$|main\.rs$$|migration/.*\.rs$$'

coverage:
	cargo llvm-cov $(COVERAGE_EXCLUDE)

coverage-html:
	cargo llvm-cov $(COVERAGE_EXCLUDE) --html

coverage-json:
	cargo llvm-cov $(COVERAGE_EXCLUDE) --json
