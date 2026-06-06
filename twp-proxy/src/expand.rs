// Phase 1 template-expansion pass.
//
// Walks the protocol tree, substituting `$param` placeholders and inlining
// `$<name>` component invocations.
//
// Semantics:
//   * `$param` nodes are replaced by their value from the lexically
//     enclosing `$call`'s `props`.
//   * `$<name>` invocations look up `<name>` in the defs map. If absent,
//     the subtree silently becomes an empty `box` (matches the protocol's
//     "graceful degradation" stance — same idea as APC drop on terminals
//     that don't understand a sequence).
//   * Props are evaluated in the *caller's* scope (call-by-value), then
//     passed as opaque data to the def. So a prop value that itself uses
//     `$param` resolves against the caller's scope, not the def's.
//   * Recursion is capped to keep a buggy or malicious def from DoSing
//     the renderer.

use std::collections::HashMap;

use crate::protocol::Node;

const MAX_DEPTH: usize = 32;

pub fn expand(scene: Node, defs: &HashMap<String, Node>) -> Node {
    let empty_scope = HashMap::new();
    let mut state = State { defs, depth: 0 };
    state.walk(scene, &empty_scope)
}

struct State<'a> {
    defs: &'a HashMap<String, Node>,
    depth: usize,
}

fn placeholder() -> Node {
    Node {
        n: "box".to_string(),
        ..Node::default()
    }
}

impl<'a> State<'a> {
    fn walk(&mut self, node: Node, scope: &HashMap<String, Node>) -> Node {
        if node.n == "$param" {
            return self.resolve_param(&node, scope);
        }

        if let Some(comp_name) = node.n.strip_prefix('$').map(str::to_string) {
            return self.invoke_component(&comp_name, node, scope);
        }

        // Plain primitive node — recurse into children in the same scope.
        let children: Vec<Node> = node
            .c
            .into_iter()
            .map(|child| self.walk(child, scope))
            .collect();
        Node {
            c: children,
            ..node
        }
    }

    fn resolve_param(&mut self, node: &Node, scope: &HashMap<String, Node>) -> Node {
        let Some(name) = &node.name else {
            return placeholder();
        };
        match scope.get(name) {
            Some(value) => value.clone(),
            None => placeholder(),
        }
    }

    fn invoke_component(
        &mut self,
        name: &str,
        invocation: Node,
        caller_scope: &HashMap<String, Node>,
    ) -> Node {
        if self.depth >= MAX_DEPTH {
            return placeholder();
        }
        let Some(def) = self.defs.get(name).cloned() else {
            return placeholder();
        };

        // Evaluate props in the caller's scope first, then build the
        // callee's scope from the resolved values.
        let mut callee_scope = HashMap::with_capacity(invocation.props.len());
        for (key, value) in invocation.props {
            let resolved = self.walk(value.into_node(), caller_scope);
            callee_scope.insert(key, resolved);
        }

        self.depth += 1;
        let result = self.walk(def, &callee_scope);
        self.depth -= 1;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Payload;

    fn run(json: &str) -> Node {
        let payload: Payload = serde_json::from_str(json).unwrap();
        expand(payload.scene.unwrap(), &payload.defs)
    }

    #[test]
    fn primitive_passes_through() {
        let n = run(r##"{"S":{"n":"box","s":{"background":"#abc"}}}"##);
        assert_eq!(n.n, "box");
        assert_eq!(n.s.background.as_deref(), Some("#abc"));
    }

    #[test]
    fn unknown_component_becomes_placeholder() {
        let n = run(r#"{"S":{"n":"$missing","props":{}}}"#);
        assert_eq!(n.n, "box");
    }

    #[test]
    fn simple_component_expansion() {
        let n = run(r#"{
                "S": {"n":"$badge","props":{"label":"PASS"}},
                "C": {
                    "badge": {"n":"box","c":[{"n":"$param","name":"label"}]}
                }
            }"#);
        assert_eq!(n.n, "box");
        assert_eq!(n.c.len(), 1);
        assert_eq!(n.c[0].n, "text");
        assert_eq!(n.c[0].t.as_deref(), Some("PASS"));
    }

    #[test]
    fn unfilled_param_becomes_placeholder() {
        let n = run(r#"{
                "S": {"n":"$badge","props":{}},
                "C": {
                    "badge": {"n":"box","c":[{"n":"$param","name":"label"}]}
                }
            }"#);
        assert_eq!(n.n, "box");
        assert_eq!(n.c[0].n, "box"); // placeholder
    }

    #[test]
    fn lexical_scope_isolation() {
        // The "outer" component supplies prop "x" to the "inner" component.
        // Inside "inner", a $param looking for "x" must find the value
        // explicitly passed in — *not* leak something from the outer scope
        // if the names happened to differ.
        let n = run(r#"{
                "S": {"n":"$outer","props":{"label":"hello"}},
                "C": {
                    "outer": {"n":"$inner","props":{"text":{"n":"$param","name":"label"}}},
                    "inner": {"n":"box","c":[{"n":"$param","name":"text"}]}
                }
            }"#);
        // outer's "label" → resolved in scene scope (empty) → wait, "label"
        // is in outer's invocation scope. So when we walk outer's body,
        // {$param name=label} resolves to "hello". outer's body invokes
        // inner with text=resolved("hello"). inner's body uses $param "text".
        assert_eq!(n.n, "box");
        assert_eq!(n.c[0].n, "text");
        assert_eq!(n.c[0].t.as_deref(), Some("hello"));
    }

    #[test]
    fn recursive_component_caps_at_max_depth() {
        // A component that invokes itself forever. Must not blow the stack
        // or hang; should bottom out at the depth cap with a placeholder.
        let n = run(r#"{
                "S": {"n":"$loop","props":{}},
                "C": {
                    "loop": {"n":"$loop","props":{}}
                }
            }"#);
        // After MAX_DEPTH invocations we return a placeholder box.
        assert_eq!(n.n, "box");
    }
}
