//! SCIM filter parser (RFC 7644 §3.4.2.2)
//!
//! Recursive descent parser for SCIM filter expressions.
//! Compiles parsed AST into SQL WHERE clauses with bind parameters.

use crate::domain::scim::{scim_attr_to_column, CompiledFilter, ScimCompareOp, ScimFilterExpr};
use crate::error::{AppError, Result};

/// Parse a SCIM filter string into an expression AST.
pub fn parse_filter(input: &str) -> Result<ScimFilterExpr> {
    let tokens = tokenize(input)?;
    let mut pos = 0;
    let expr = parse_or(&tokens, &mut pos)?;
    if pos < tokens.len() {
        return Err(AppError::BadRequest(format!(
            "Unexpected token at position {}: '{}'",
            pos, tokens[pos]
        )));
    }
    Ok(expr)
}

/// Compile a filter expression into a SQL WHERE clause.
pub fn compile_filter(expr: &ScimFilterExpr) -> Result<CompiledFilter> {
    let mut bindings = Vec::new();
    let where_clause = compile_expr(expr, &mut bindings)?;
    Ok(CompiledFilter {
        where_clause,
        bindings,
    })
}

fn compile_expr(expr: &ScimFilterExpr, bindings: &mut Vec<String>) -> Result<String> {
    match expr {
        ScimFilterExpr::Compare { attr, op, value } => {
            let column = scim_attr_to_column(attr).ok_or_else(|| {
                AppError::BadRequest(format!("Unsupported SCIM filter attribute: {}", attr))
            })?;

            // Special handling for "active" attribute
            if column == "users.locked_until" {
                let is_active = value.to_lowercase() == "true";
                return Ok(if is_active {
                    "users.locked_until IS NULL".to_string()
                } else {
                    "users.locked_until IS NOT NULL".to_string()
                });
            }

            let sql = match op {
                ScimCompareOp::Eq => {
                    bindings.push(value.clone());
                    format!("{} = ?", column)
                }
                ScimCompareOp::Ne => {
                    bindings.push(value.clone());
                    format!("{} != ?", column)
                }
                ScimCompareOp::Co => {
                    bindings.push(format!("%{}%", value));
                    format!("{} LIKE ?", column)
                }
                ScimCompareOp::Sw => {
                    bindings.push(format!("{}%", value));
                    format!("{} LIKE ?", column)
                }
                ScimCompareOp::Ew => {
                    bindings.push(format!("%{}", value));
                    format!("{} LIKE ?", column)
                }
                ScimCompareOp::Gt => {
                    bindings.push(value.clone());
                    format!("{} > ?", column)
                }
                ScimCompareOp::Ge => {
                    bindings.push(value.clone());
                    format!("{} >= ?", column)
                }
                ScimCompareOp::Lt => {
                    bindings.push(value.clone());
                    format!("{} < ?", column)
                }
                ScimCompareOp::Le => {
                    bindings.push(value.clone());
                    format!("{} <= ?", column)
                }
            };
            Ok(sql)
        }
        ScimFilterExpr::Present { attr } => {
            let column = scim_attr_to_column(attr).ok_or_else(|| {
                AppError::BadRequest(format!("Unsupported SCIM filter attribute: {}", attr))
            })?;
            if column == "users.locked_until" {
                Ok("users.locked_until IS NULL".to_string())
            } else {
                Ok(format!("{} IS NOT NULL", column))
            }
        }
        ScimFilterExpr::And(left, right) => {
            let l = compile_expr(left, bindings)?;
            let r = compile_expr(right, bindings)?;
            Ok(format!("({} AND {})", l, r))
        }
        ScimFilterExpr::Or(left, right) => {
            let l = compile_expr(left, bindings)?;
            let r = compile_expr(right, bindings)?;
            Ok(format!("({} OR {})", l, r))
        }
        ScimFilterExpr::Not(inner) => {
            let i = compile_expr(inner, bindings)?;
            Ok(format!("NOT ({})", i))
        }
    }
}

// ============================================================
// Tokenizer
// ============================================================

fn tokenize(input: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        if chars[i] == '(' {
            tokens.push("(".to_string());
            i += 1;
        } else if chars[i] == ')' {
            tokens.push(")".to_string());
            i += 1;
        } else if chars[i] == '"' {
            // Quoted string
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    i += 1; // skip escape
                }
                i += 1;
            }
            if i >= chars.len() {
                return Err(AppError::BadRequest(
                    "Unterminated string in SCIM filter".to_string(),
                ));
            }
            let value: String = chars[start..i].iter().collect();
            tokens.push(format!("\"{}\"", value));
            i += 1; // skip closing quote
        } else {
            // Unquoted token (attribute name, operator, or value)
            let start = i;
            while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '(' && chars[i] != ')'
            {
                i += 1;
            }
            let token: String = chars[start..i].iter().collect();
            tokens.push(token);
        }
    }
    Ok(tokens)
}

// ============================================================
// Recursive Descent Parser
// ============================================================

fn parse_or(tokens: &[String], pos: &mut usize) -> Result<ScimFilterExpr> {
    let mut left = parse_and(tokens, pos)?;
    while *pos < tokens.len() && tokens[*pos].to_lowercase() == "or" {
        *pos += 1;
        let right = parse_and(tokens, pos)?;
        left = ScimFilterExpr::Or(Box::new(left), Box::new(right));
    }
    Ok(left)
}

fn parse_and(tokens: &[String], pos: &mut usize) -> Result<ScimFilterExpr> {
    let mut left = parse_not(tokens, pos)?;
    while *pos < tokens.len() && tokens[*pos].to_lowercase() == "and" {
        *pos += 1;
        let right = parse_not(tokens, pos)?;
        left = ScimFilterExpr::And(Box::new(left), Box::new(right));
    }
    Ok(left)
}

fn parse_not(tokens: &[String], pos: &mut usize) -> Result<ScimFilterExpr> {
    if *pos < tokens.len() && tokens[*pos].to_lowercase() == "not" {
        *pos += 1;
        let inner = parse_atom(tokens, pos)?;
        Ok(ScimFilterExpr::Not(Box::new(inner)))
    } else {
        parse_atom(tokens, pos)
    }
}

fn parse_atom(tokens: &[String], pos: &mut usize) -> Result<ScimFilterExpr> {
    if *pos >= tokens.len() {
        return Err(AppError::BadRequest(
            "Unexpected end of SCIM filter".to_string(),
        ));
    }

    // Parenthesized expression
    if tokens[*pos] == "(" {
        *pos += 1;
        let expr = parse_or(tokens, pos)?;
        if *pos >= tokens.len() || tokens[*pos] != ")" {
            return Err(AppError::BadRequest(
                "Missing closing parenthesis in SCIM filter".to_string(),
            ));
        }
        *pos += 1;
        return Ok(expr);
    }

    // Must be: attr op value  OR  attr pr
    let attr = tokens[*pos].clone();
    *pos += 1;

    if *pos >= tokens.len() {
        return Err(AppError::BadRequest(format!(
            "Expected operator after '{}' in SCIM filter",
            attr
        )));
    }

    let op_str = tokens[*pos].to_lowercase();
    if op_str == "pr" {
        *pos += 1;
        return Ok(ScimFilterExpr::Present { attr });
    }

    let op = ScimCompareOp::parse(&op_str).ok_or_else(|| {
        AppError::BadRequest(format!("Unknown SCIM filter operator: '{}'", op_str))
    })?;
    *pos += 1;

    if *pos >= tokens.len() {
        return Err(AppError::BadRequest(format!(
            "Expected value after '{}' in SCIM filter",
            op_str
        )));
    }

    let value_token = &tokens[*pos];
    let value = if value_token.starts_with('"') && value_token.ends_with('"') {
        value_token[1..value_token.len() - 1].to_string()
    } else {
        value_token.clone()
    };
    *pos += 1;

    Ok(ScimFilterExpr::Compare { attr, op, value })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_eq() {
        let expr = parse_filter("userName eq \"john@example.com\"").unwrap();
        assert_eq!(
            expr,
            ScimFilterExpr::Compare {
                attr: "userName".to_string(),
                op: ScimCompareOp::Eq,
                value: "john@example.com".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_starts_with() {
        let expr = parse_filter("userName sw \"john\"").unwrap();
        assert_eq!(
            expr,
            ScimFilterExpr::Compare {
                attr: "userName".to_string(),
                op: ScimCompareOp::Sw,
                value: "john".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_contains() {
        let expr = parse_filter("displayName co \"smith\"").unwrap();
        assert_eq!(
            expr,
            ScimFilterExpr::Compare {
                attr: "displayName".to_string(),
                op: ScimCompareOp::Co,
                value: "smith".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_present() {
        let expr = parse_filter("displayName pr").unwrap();
        assert_eq!(
            expr,
            ScimFilterExpr::Present {
                attr: "displayName".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_and() {
        let expr = parse_filter("userName eq \"john\" and active eq \"true\"").unwrap();
        match expr {
            ScimFilterExpr::And(left, right) => {
                assert!(matches!(*left, ScimFilterExpr::Compare { .. }));
                assert!(matches!(*right, ScimFilterExpr::Compare { .. }));
            }
            _ => panic!("Expected And expression"),
        }
    }

    #[test]
    fn test_parse_or() {
        let expr = parse_filter("userName eq \"a\" or userName eq \"b\"").unwrap();
        assert!(matches!(expr, ScimFilterExpr::Or(_, _)));
    }

    #[test]
    fn test_parse_not() {
        let expr = parse_filter("not userName eq \"test\"").unwrap();
        assert!(matches!(expr, ScimFilterExpr::Not(_)));
    }

    #[test]
    fn test_parse_parentheses() {
        let expr = parse_filter("(userName eq \"a\" or userName eq \"b\") and active eq \"true\"")
            .unwrap();
        assert!(matches!(expr, ScimFilterExpr::And(_, _)));
    }

    #[test]
    fn test_parse_nested_and_or() {
        let expr = parse_filter("userName eq \"a\" and displayName eq \"b\" or active eq \"true\"")
            .unwrap();
        // OR has lower precedence, so this is: (userName eq "a" and displayName eq "b") or (active eq "true")
        assert!(matches!(expr, ScimFilterExpr::Or(_, _)));
    }

    #[test]
    fn test_parse_invalid_operator() {
        let result = parse_filter("userName xx \"test\"");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unterminated_string() {
        let result = parse_filter("userName eq \"unterminated");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_value() {
        let result = parse_filter("userName eq");
        assert!(result.is_err());
    }

    #[test]
    fn test_compile_eq() {
        let expr = parse_filter("userName eq \"john@example.com\"").unwrap();
        let compiled = compile_filter(&expr).unwrap();
        assert_eq!(compiled.where_clause, "users.email = ?");
        assert_eq!(compiled.bindings, vec!["john@example.com"]);
    }

    #[test]
    fn test_compile_contains() {
        let expr = parse_filter("displayName co \"smith\"").unwrap();
        let compiled = compile_filter(&expr).unwrap();
        assert_eq!(compiled.where_clause, "users.display_name LIKE ?");
        assert_eq!(compiled.bindings, vec!["%smith%"]);
    }

    #[test]
    fn test_compile_starts_with() {
        let expr = parse_filter("userName sw \"john\"").unwrap();
        let compiled = compile_filter(&expr).unwrap();
        assert_eq!(compiled.where_clause, "users.email LIKE ?");
        assert_eq!(compiled.bindings, vec!["john%"]);
    }

    #[test]
    fn test_compile_ends_with() {
        let expr = parse_filter("userName ew \"@example.com\"").unwrap();
        let compiled = compile_filter(&expr).unwrap();
        assert_eq!(compiled.where_clause, "users.email LIKE ?");
        assert_eq!(compiled.bindings, vec!["%@example.com"]);
    }

    #[test]
    fn test_compile_active_true() {
        let expr = parse_filter("active eq \"true\"").unwrap();
        let compiled = compile_filter(&expr).unwrap();
        assert_eq!(compiled.where_clause, "users.locked_until IS NULL");
        assert!(compiled.bindings.is_empty());
    }

    #[test]
    fn test_compile_active_false() {
        let expr = parse_filter("active eq \"false\"").unwrap();
        let compiled = compile_filter(&expr).unwrap();
        assert_eq!(compiled.where_clause, "users.locked_until IS NOT NULL");
        assert!(compiled.bindings.is_empty());
    }

    #[test]
    fn test_compile_present() {
        let expr = parse_filter("displayName pr").unwrap();
        let compiled = compile_filter(&expr).unwrap();
        assert_eq!(compiled.where_clause, "users.display_name IS NOT NULL");
    }

    #[test]
    fn test_compile_and() {
        let expr = parse_filter("userName eq \"john\" and active eq \"true\"").unwrap();
        let compiled = compile_filter(&expr).unwrap();
        assert_eq!(
            compiled.where_clause,
            "(users.email = ? AND users.locked_until IS NULL)"
        );
        assert_eq!(compiled.bindings, vec!["john"]);
    }

    #[test]
    fn test_compile_or() {
        let expr = parse_filter("userName eq \"a\" or userName eq \"b\"").unwrap();
        let compiled = compile_filter(&expr).unwrap();
        assert_eq!(
            compiled.where_clause,
            "(users.email = ? OR users.email = ?)"
        );
        assert_eq!(compiled.bindings, vec!["a", "b"]);
    }

    #[test]
    fn test_compile_not() {
        let expr = parse_filter("not active eq \"true\"").unwrap();
        let compiled = compile_filter(&expr).unwrap();
        assert_eq!(compiled.where_clause, "NOT (users.locked_until IS NULL)");
    }

    #[test]
    fn test_compile_ne() {
        let expr = parse_filter("userName ne \"john\"").unwrap();
        let compiled = compile_filter(&expr).unwrap();
        assert_eq!(compiled.where_clause, "users.email != ?");
        assert_eq!(compiled.bindings, vec!["john"]);
    }

    #[test]
    fn test_compile_gt_ge_lt_le() {
        let cases = vec![
            ("id gt \"100\"", "users.id > ?"),
            ("id ge \"100\"", "users.id >= ?"),
            ("id lt \"100\"", "users.id < ?"),
            ("id le \"100\"", "users.id <= ?"),
        ];
        for (filter, expected) in cases {
            let expr = parse_filter(filter).unwrap();
            let compiled = compile_filter(&expr).unwrap();
            assert_eq!(compiled.where_clause, expected, "Filter: {}", filter);
        }
    }

    #[test]
    fn test_compile_unsupported_attribute() {
        let expr = parse_filter("unknownAttr eq \"value\"").unwrap();
        let result = compile_filter(&expr);
        assert!(result.is_err());
    }

    #[test]
    fn test_compile_external_id() {
        let expr = parse_filter("externalId eq \"ext-123\"").unwrap();
        let compiled = compile_filter(&expr).unwrap();
        assert_eq!(compiled.where_clause, "users.scim_external_id = ?");
        assert_eq!(compiled.bindings, vec!["ext-123"]);
    }

    #[test]
    fn test_unquoted_value() {
        let expr = parse_filter("userName eq john@example.com").unwrap();
        assert_eq!(
            expr,
            ScimFilterExpr::Compare {
                attr: "userName".to_string(),
                op: ScimCompareOp::Eq,
                value: "john@example.com".to_string(),
            }
        );
    }

    #[test]
    fn test_active_present() {
        let expr = parse_filter("active pr").unwrap();
        let compiled = compile_filter(&expr).unwrap();
        assert_eq!(compiled.where_clause, "users.locked_until IS NULL");
    }
}
