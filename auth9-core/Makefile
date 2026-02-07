.PHONY: coverage coverage-html coverage-json
.PHONY: coverage-lib coverage-lib-html coverage-lib-json

# Files excluded from coverage tracking:
# - repository/*.rs: Thin data mapping layer (ORM-equivalent), no business logic
# - main.rs: Program entry point
# - migration/*.rs: Database migration scripts
COVERAGE_EXCLUDE := --ignore-filename-regex 'repository/.*\.rs$$|main\.rs$$|migration/.*\.rs$$'

# Lib-only coverage is useful in restricted environments where integration tests cannot
# bind local TCP ports (e.g., wiremock-based tests).
# It also excludes thin wiring/integration layers to keep the metric focused on business logic.
COVERAGE_EXCLUDE_LIB := --ignore-filename-regex 'repository/.*\.rs$$|main\.rs$$|migration/.*\.rs$$|keycloak/client\.rs$$|keycloak/seeder\.rs$$|server/mod\.rs$$'

coverage:
	cargo llvm-cov $(COVERAGE_EXCLUDE)

coverage-html:
	cargo llvm-cov $(COVERAGE_EXCLUDE) --html

coverage-json:
	cargo llvm-cov $(COVERAGE_EXCLUDE) --json

coverage-lib:
	cargo llvm-cov $(COVERAGE_EXCLUDE_LIB) --lib

coverage-lib-html:
	cargo llvm-cov $(COVERAGE_EXCLUDE_LIB) --lib --html

coverage-lib-json:
	cargo llvm-cov $(COVERAGE_EXCLUDE_LIB) --lib --json
