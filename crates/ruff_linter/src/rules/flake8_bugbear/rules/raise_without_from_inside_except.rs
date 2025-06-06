use ruff_python_ast as ast;
use ruff_python_ast::Stmt;

use ruff_macros::{ViolationMetadata, derive_message_formats};
use ruff_python_ast::helpers::RaiseStatementVisitor;
use ruff_python_ast::statement_visitor::StatementVisitor;

use crate::Violation;
use crate::checkers::ast::Checker;

/// ## What it does
/// Checks for `raise` statements in exception handlers that lack a `from`
/// clause.
///
/// ## Why is this bad?
/// In Python, `raise` can be used with or without an exception from which the
/// current exception is derived. This is known as exception chaining. When
/// printing the stack trace, chained exceptions are displayed in such a way
/// so as make it easier to trace the exception back to its root cause.
///
/// When raising an exception from within an `except` clause, always include a
/// `from` clause to facilitate exception chaining. If the exception is not
/// chained, it will be difficult to trace the exception back to its root cause.
///
/// ## Example
/// ```python
/// try:
///     ...
/// except FileNotFoundError:
///     if ...:
///         raise RuntimeError("...")
///     else:
///         raise UserWarning("...")
/// ```
///
/// Use instead:
/// ```python
/// try:
///     ...
/// except FileNotFoundError as exc:
///     if ...:
///         raise RuntimeError("...") from None
///     else:
///         raise UserWarning("...") from exc
/// ```
///
/// ## References
/// - [Python documentation: `raise` statement](https://docs.python.org/3/reference/simple_stmts.html#the-raise-statement)
#[derive(ViolationMetadata)]
pub(crate) struct RaiseWithoutFromInsideExcept {
    is_star: bool,
}

impl Violation for RaiseWithoutFromInsideExcept {
    #[derive_message_formats]
    fn message(&self) -> String {
        if self.is_star {
            "Within an `except*` clause, raise exceptions with `raise ... from err` or `raise ... \
                 from None` to distinguish them from errors in exception handling"
                .to_string()
        } else {
            "Within an `except` clause, raise exceptions with `raise ... from err` or `raise ... \
                 from None` to distinguish them from errors in exception handling"
                .to_string()
        }
    }
}

/// B904
pub(crate) fn raise_without_from_inside_except(
    checker: &Checker,
    name: Option<&str>,
    body: &[Stmt],
) {
    let raises = {
        let mut visitor = RaiseStatementVisitor::default();
        visitor.visit_body(body);
        visitor.raises
    };

    for (range, exc, cause) in raises {
        if cause.is_none() {
            if let Some(exc) = exc {
                // If the raised object is bound to the same name, it's a re-raise, which is
                // allowed (but may be flagged by other diagnostics).
                //
                // For example:
                // ```python
                // try:
                //     ...
                // except ValueError as exc:
                //     raise exc
                // ```
                if let Some(name) = name {
                    if exc
                        .as_name_expr()
                        .is_some_and(|ast::ExprName { id, .. }| name == id)
                    {
                        continue;
                    }
                }

                let is_star = checker
                    .semantic()
                    .current_statement()
                    .as_try_stmt()
                    .is_some_and(|try_stmt| try_stmt.is_star);

                checker.report_diagnostic(RaiseWithoutFromInsideExcept { is_star }, range);
            }
        }
    }
}
