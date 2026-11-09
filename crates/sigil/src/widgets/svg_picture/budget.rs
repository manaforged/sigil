use std::collections::HashMap;

use resvg::usvg::roxmltree::{Document, Node};
use resvg::usvg::{Group, Node as TreeNode, Tree};

/// The most scene commands one picture lowers to before it is drawn as one resvg bitmap.
pub(super) const MAX_COMMANDS: usize = 65_536;
const MAX_NODES: usize = 16_384;
const MAX_ELEMENT_DEPTH: usize = 128;
const MAX_CHAIN: usize = 32;
const MAX_REFERENCED_DEPTH: usize = 256;
const MAX_EXPANSION: usize = 65_536;
const DEFINITIONS: [&str; 5] = ["clipPath", "mask", "pattern", "marker", "filter"];

/// Whether the nodes of `tree`, with every clip, mask, pattern and text subtree counted at each use, fit the budget.
pub(super) fn fits(tree: &Tree) -> bool {
    let mut left = MAX_NODES;
    within(tree.root(), &mut left)
}

fn within(group: &Group, left: &mut usize) -> bool {
    for node in group.children() {
        let Some(rest) = left.checked_sub(1) else {
            return false;
        };
        *left = rest;
        let mut fits = true;
        node.subroots(|root| fits = fits && within(root, left));
        if !fits {
            return false;
        }
        if let TreeNode::Group(child) = node
            && !within(child, left)
        {
            return false;
        }
    }
    true
}

/// Whether a parsed SVG is shallow enough, and its references short and narrow enough, for usvg, which recurses per element level and per link of a reference chain and copies a definition at each use.
pub(super) fn document_fits(document: &Document<'_>) -> bool {
    let deepest = deepest(document);
    if deepest > MAX_ELEMENT_DEPTH {
        return false;
    }
    let reach = Links::of(document).reach();
    reach.chain <= MAX_CHAIN
        && reach.expansion <= MAX_EXPANSION
        && deepest.saturating_mul(reach.chain + 1) <= MAX_REFERENCED_DEPTH
}

fn deepest(document: &Document<'_>) -> usize {
    let mut depths = vec![0_usize; document.descendants().count() + 1];
    let mut deepest = 0;
    for node in document.descendants() {
        let parent = node
            .parent()
            .and_then(|parent| depths.get(parent.id().get_usize()).copied())
            .unwrap_or(0);
        let depth = parent + usize::from(node.is_element() && node.has_children());
        if let Some(slot) = depths.get_mut(node.id().get_usize()) {
            *slot = depth;
        }
        deepest = deepest.max(depth);
    }
    deepest
}

fn targets(value: &str, out: &mut Vec<String>) {
    for (at, _) in value.match_indices("url(") {
        let rest = value[at + 4..].trim_start_matches([' ', '"', '\'']);
        if let Some(id) = rest.strip_prefix('#') {
            let end = id.find([')', ' ', '"', '\'']).unwrap_or(id.len());
            out.push(id[..end].to_string());
        }
    }
}

#[derive(Default)]
struct Rules {
    classes: HashMap<String, Vec<String>>,
    ids: HashMap<String, Vec<String>>,
    any: Vec<String>,
}

impl Rules {
    fn of(document: &Document<'_>) -> Self {
        let mut rules = Self::default();
        let sheets = document
            .descendants()
            .filter(|node| node.tag_name().name() == "style")
            .filter_map(|node| node.text());
        for sheet in sheets {
            for rule in sheet.split('}') {
                if let Some((selectors, body)) = rule.split_once('{') {
                    rules.add(selectors, body);
                }
            }
        }
        rules
    }

    fn add(&mut self, selectors: &str, body: &str) {
        let mut found = Vec::new();
        targets(body, &mut found);
        if found.is_empty() {
            return;
        }
        for selector in selectors.split(',') {
            let last = selector.split_whitespace().last().unwrap_or_default();
            let bucket = match (last.split_once('.'), last.split_once('#')) {
                (Some((_, class)), _) => self.classes.entry(class.to_string()).or_default(),
                (_, Some((_, id))) => self.ids.entry(id.to_string()).or_default(),
                _ => &mut self.any,
            };
            bucket.extend(found.iter().cloned());
        }
    }

    fn of_element(&self, node: Node<'_, '_>, out: &mut Vec<String>) {
        for class in node
            .attribute("class")
            .unwrap_or_default()
            .split_whitespace()
        {
            out.extend(self.classes.get(class).into_iter().flatten().cloned());
        }
        if let Some(found) = node.attribute("id").and_then(|id| self.ids.get(id)) {
            out.extend(found.iter().cloned());
        }
    }
}

struct Reach {
    chain: usize,
    expansion: usize,
}

struct Links {
    edges: Vec<Vec<usize>>,
    unscoped: usize,
}

impl Links {
    fn of(document: &Document<'_>) -> Self {
        let elements = || document.descendants().filter(Node::is_element);
        let ids: HashMap<&str, usize> = elements()
            .filter_map(|node| node.attribute("id"))
            .enumerate()
            .map(|(index, id)| (id, index))
            .collect();
        let rules = Rules::of(document);
        let mut edges = vec![Vec::new(); ids.len()];
        for node in elements() {
            let found: Vec<usize> = references(node, &rules)
                .iter()
                .filter_map(|id| ids.get(id.as_str()).copied())
                .collect();
            if found.is_empty() {
                continue;
            }
            for holder in node.ancestors().filter_map(|node| node.attribute("id")) {
                if let Some(outgoing) = ids.get(holder).and_then(|&holder| edges.get_mut(holder)) {
                    outgoing.extend(&found);
                }
            }
        }
        let names_definition = |id: &&String| {
            elements().any(|node| {
                node.attribute("id") == Some(id.as_str())
                    && DEFINITIONS.contains(&node.tag_name().name())
            })
        };
        let unscoped = rules.any.iter().filter(names_definition).count();
        Self { edges, unscoped }
    }

    fn reach(&self) -> Reach {
        let count = self.edges.len();
        let mut walk = Walk::new(count);
        let mut cycles = 0_usize;
        for start in 0..count {
            if walk.state(start) != Visit::New {
                continue;
            }
            walk.mark(start, Visit::Open);
            let mut stack = vec![(start, 0_usize)];
            while let Some(&mut (node, ref mut next)) = stack.last_mut() {
                let Some(&target) = self.edges.get(node).and_then(|edges| edges.get(*next)) else {
                    walk.mark(node, Visit::Done);
                    stack.pop();
                    if let Some(&(parent, _)) = stack.last() {
                        walk.absorb(parent, node);
                    }
                    continue;
                };
                *next += 1;
                match walk.state(target) {
                    Visit::New => {
                        walk.mark(target, Visit::Open);
                        stack.push((target, 0));
                    }
                    Visit::Open => cycles += 1,
                    Visit::Done => walk.absorb(node, target),
                }
            }
        }
        let longest = walk.chain.into_iter().max().unwrap_or(0);
        Reach {
            chain: (cycles + 1).saturating_mul(longest + 1) - 1 + self.unscoped,
            expansion: walk.expansion.into_iter().max().unwrap_or(0),
        }
    }
}

struct Walk {
    seen: Vec<Visit>,
    chain: Vec<usize>,
    expansion: Vec<usize>,
}

impl Walk {
    fn new(count: usize) -> Self {
        Self {
            seen: vec![Visit::New; count],
            chain: vec![0; count],
            expansion: vec![0; count],
        }
    }

    fn state(&self, node: usize) -> Visit {
        self.seen.get(node).copied().unwrap_or(Visit::Done)
    }

    fn mark(&mut self, node: usize, visit: Visit) {
        if let Some(slot) = self.seen.get_mut(node) {
            *slot = visit;
        }
    }

    fn absorb(&mut self, into: usize, from: usize) {
        let (Some(&chain), Some(&expansion)) = (self.chain.get(from), self.expansion.get(from))
        else {
            return;
        };
        if let Some(slot) = self.chain.get_mut(into) {
            *slot = (*slot).max(chain + 1);
        }
        if let Some(slot) = self.expansion.get_mut(into) {
            *slot = slot.saturating_add(expansion).saturating_add(1);
        }
    }
}

fn references(node: Node<'_, '_>, rules: &Rules) -> Vec<String> {
    let mut found = Vec::new();
    for attribute in node.attributes() {
        targets(attribute.value(), &mut found);
        if attribute.name() == "href"
            && let Some(id) = attribute.value().strip_prefix('#')
        {
            found.push(id.to_string());
        }
    }
    rules.of_element(node, &mut found);
    found
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Visit {
    New,
    Open,
    Done,
}
