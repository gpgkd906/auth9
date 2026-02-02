# QA Testing Tickets

This directory contains individual ticket files for failed QA test scenarios.

## Ticket Format

Each ticket file represents a single failed test scenario with the following information:

- **测试内容** (Test Content): What was being tested
- **预期结果** (Expected Result): What should have happened
- **再现方法** (Reproduction Steps): How to reproduce the issue
- **实际结果** (Actual Result): What actually happened, including error logs and database state

## Naming Convention

Ticket files follow this naming pattern:

```
{module}_{document}_scenario{N}_{YYMMDD_HHMMSS}.md
```

Examples:
- `user_01-crud_scenario2_260203_143052.md`
- `tenant_01-crud_scenario5_260203_143125.md`
- `rbac_02-role_scenario3_260203_143201.md`

## Workflow

1. **QA Testing Skill** automatically creates tickets when scenarios fail
2. Development team reviews tickets to understand issues
3. Developers fix the issues based on ticket details
4. Re-run QA tests to verify fixes
5. Archive or delete resolved tickets

## Ticket Lifecycle

```
[FAILED] → [IN_PROGRESS] → [FIXED] → [VERIFIED] → [CLOSED]
```

Update ticket status by editing the **Status** field in the ticket file.

## Related Directories

- `docs/qa/` - QA test scenarios organized by module
- `docs/report/` - (Deprecated) Old comprehensive test reports
