use clippy_utils::diagnostics::span_lint_and_sugg;
use rustc_ast::ast::*;
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// ### Why is this bad?
    ///
    /// ### Example
    /// ```rust
    /// // example code where clippy issues a warning
    /// ```
    /// Use instead:
    /// ```rust
    /// // example code which does not raise clippy warning
    /// ```
    #[clippy::version = "1.58.0"]
    pub ORDER_OF_USE,
    nursery,
    "default lint description"
}
declare_lint_pass!(OrderOfUse => [ORDER_OF_USE]);

impl EarlyLintPass for OrderOfUse {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &Item) {
        if let ItemKind::Use(use_tree) = &item.kind {
            // TODO: Dummy
            let span = item.span;
            let std_block = vec![use_tree];
            let other_block = vec![use_tree];
            let crate_block = vec![use_tree];
            suggest_use_blocks(cx, span, &std_block, &other_block, &crate_block);
        }
    }
}

fn suggest_use_blocks(
    cx: &EarlyContext<'_>,
    span: Span,
    std_use: &[&UseTree],
    other_use: &[&UseTree],
    crate_use: &[&UseTree],
) {
    let sugg = format!(
        "{}\n\n{}\n\n{}\n",
        std_use.to_code(),
        other_use.to_code(),
        crate_use.to_code()
    );
    span_lint_and_sugg(
        cx,
        ORDER_OF_USE,
        span,
        "Put a block of `use` here",
        "try",
        sugg,
        Applicability::MachineApplicable,
    );
}

trait ToCode {
    fn to_code(&self) -> String;
}

impl ToCode for &[&UseTree] {
    fn to_code(&self) -> String {
        self.iter()
            .map(|use_tree| format!("use {};", use_tree.to_code()))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl ToCode for UseTree {
    fn to_code(&self) -> String {
        let prefix = self.prefix.to_code();
        match &self.kind {
            UseTreeKind::Simple(None, _, _) => {
                format!("{}", prefix)
            },
            UseTreeKind::Simple(Some(alias), _, _) => {
                format!("{} as {}", prefix, alias.to_string())
            },
            UseTreeKind::Nested(vec) => {
                let children = vec.iter().map(|(u, _)| u.to_code()).collect::<Vec<_>>().join(", ");
                if prefix.is_empty() {
                    format!("{{{}}}", children)
                } else {
                    format!("{}::{{{}}}", prefix, children)
                }
            },
            UseTreeKind::Glob => {
                if prefix.is_empty() {
                    format!("*")
                } else {
                    format!("{}::*", prefix)
                }
            },
        }
    }
}

impl ToCode for Path {
    fn to_code(&self) -> String {
        self.segments
            .iter()
            .map(|seg| seg.ident.name.to_string())
            .collect::<Vec<_>>()
            .join("::")
    }
}
