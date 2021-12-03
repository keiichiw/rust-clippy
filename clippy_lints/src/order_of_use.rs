use clippy_utils::diagnostics::span_lint_and_sugg;
use rustc_ast::ast::*;
use rustc_errors::Applicability;
use rustc_ast::ptr::P;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::{BytePos, Span};


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
    style,
    "default lint description"
}
declare_lint_pass!(OrderOfUse => [ORDER_OF_USE]);

impl EarlyLintPass for OrderOfUse {
    fn check_item(&mut self, ectx: &EarlyContext<'_>, item: &Item) {
        match item.kind {
            ItemKind::Mod(_, ModKind::Loaded(ref mod_items, _, _)) => check_use_order(ectx, &**mod_items),
            _ => (),
        }
    }
}

fn check_use_order<'ecx>(ectx: &EarlyContext<'ecx>, mod_items: &[P<Item>]) {
    let use_trees = collect_use_trees(mod_items);
    if !check_use_trees(ectx, &use_trees) {
        return;
    }

    let (std_vec, other_vec, crate_vec) = make_groups(&use_trees);
    let use_block_span = find_use_block_span(mod_items);
    if let Some(span) = use_block_span {
        suggest_use_blocks(ectx, span, &std_vec, &other_vec, &crate_vec)
    }
}

fn collect_use_trees(mod_items: &[P<Item>]) -> Vec<&UseTree> {
    let mut result = mod_items.iter()
        .filter_map(|item| match (&**item).kind {
            ItemKind::Use(ref use_tree) => Some(use_tree),
            _ => None,
        }).collect::<Vec<&UseTree>>();
    result.sort_by(|a, b| a.span.partial_cmp(&b.span).unwrap());
    result
}

fn find_use_block_span(mod_items: &[P<Item>]) -> Option<Span> {
    let mut result = None;
    for item in mod_items.iter() {
        match (&**item).kind {
            ItemKind::Use(_) => result = Some(match result {
                None => item.span,
                Some(span) => merge_span(span, item.span)
            }),
            _ => (),
        }
    }
    result
}

fn merge_span(span1: Span, span2: Span) -> Span {
    Span::new(BytePos(std::cmp::min(span1.lo().0, span2.lo().0)),
              BytePos(std::cmp::max(span1.hi().0, span2.hi().0)),
              span1.ctxt(),
              None)
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Category {
    SuperOrCrate,
    Std,
    Other,
}

fn category_order(category: Category) -> usize {
    match category {
        Category::Std => 0,
        Category::Other => 1,
        Category::SuperOrCrate => 2,
    }
}

fn find_category(use_tree: &UseTree) -> Option<Category> {
    if use_tree.prefix.segments.len() == 0 {
        None
    } else {
        Some(match &*use_tree.prefix.segments[0].ident.as_str() {
            "super" | "crate" => Category::SuperOrCrate,
            "std" => Category::Std,
            _ => Category::Other,
        })
    }
}

fn is_adjuscent<'ecx>(ectx: &EarlyContext<'ecx>, span1: &Span, span2: &Span) -> bool {
    let loc1 = ectx.sess.source_map().span_to_lines(*span1).expect("Failed to find line info");
    let loc2 = ectx.sess.source_map().span_to_lines(*span2).expect("Failed to find line info");
    let loc1_max = loc1.lines.iter().map(|info| info.line_index).max().unwrap();
    let loc2_min = loc2.lines.iter().map(|info| info.line_index).min().unwrap();
    loc1_max + 1 >= loc2_min
}

// Returns whether we should run autofix.
fn check_use_trees<'ecx>(ectx: &EarlyContext<'ecx>, use_trees: &[&UseTree]) -> bool {
    if use_trees.len() == 0 {
        return false;
    }

    let mut has_error = false;
    let mut iter = use_trees.iter().peekable();
    loop {
        let prev_use_tree = *iter.next().unwrap();
        let next = iter.peek();
        if next.is_none() {
            // End of use block.
            return has_error;
        }
        let next_use_tree = *next.unwrap();
        let cat1 = find_category(prev_use_tree);
        let cat2 = find_category(next_use_tree);
        if cat1.is_none() || cat2.is_none() {
            return false;
        }
        let cat1 = cat1.unwrap();
        let cat2 = cat2.unwrap();
        if cat1 == cat2 {
            if !is_adjuscent(ectx, &prev_use_tree.span, &next_use_tree.span) {
                // eprint!("Lint error found!! Found empty line separation in a group: {:?}\n", prev_use_tree.span);
                has_error = true;
            }
        } else {
            if category_order(cat1) > category_order(cat2) {
                // eprint!("Lint Error found!! The order of the use groups is not following style guide: {:?}\n", prev_use_tree.span);
                has_error = true;
            } else if is_adjuscent(ectx, &prev_use_tree.span, &next_use_tree.span) {
                // eprint!("Lint Error found!! Different use groups should be separated by an empty line: {:?}\n", prev_use_tree.span);
                has_error = true;
            }
        }
    }
}

fn make_groups<'a>(use_trees: &[&'a UseTree]) -> (Vec<&'a UseTree>, Vec<&'a UseTree>, Vec<&'a UseTree>) {
    let mut std_vec = vec![];
    let mut other_vec = vec![];
    let mut crate_vec = vec![];

    for &use_tree in use_trees.iter() {
        match find_category(use_tree).unwrap() {
            Category::Std => std_vec.push(use_tree),
            Category::Other => other_vec.push(use_tree),
            Category::SuperOrCrate => crate_vec.push(use_tree),
        }
    }

    (std_vec, other_vec, crate_vec)
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
