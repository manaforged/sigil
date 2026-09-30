use std::collections::HashMap;

use accesskit::{Action, NodeId as AkNodeId, Role, Toggled};
use sigil::geometry::{Point, Rect};

use super::Harness;

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    id: AkNodeId,
    role: Role,
    name: Option<String>,
    value: Option<String>,
    numeric: Option<f64>,
    bounds: Option<Rect>,
    focused: bool,
    checked: Option<bool>,
    selected: Option<bool>,
    expanded: Option<bool>,
    disabled: bool,
    control: bool,
}

impl Node {
    #[must_use]
    pub const fn role(&self) -> Role {
        self.role
    }

    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    #[must_use]
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    #[must_use]
    pub const fn numeric(&self) -> Option<f64> {
        self.numeric
    }

    #[must_use]
    pub const fn bounds(&self) -> Option<Rect> {
        self.bounds
    }

    #[must_use]
    pub fn center(&self) -> Option<Point> {
        self.bounds.map(|rect| {
            Point::new(
                rect.origin.x + rect.width / 2.0,
                rect.origin.y + rect.height / 2.0,
            )
        })
    }

    #[must_use]
    pub const fn is_focused(&self) -> bool {
        self.focused
    }

    #[must_use]
    pub const fn is_checked(&self) -> Option<bool> {
        self.checked
    }

    #[must_use]
    pub const fn is_selected(&self) -> Option<bool> {
        self.selected
    }

    #[must_use]
    pub const fn is_expanded(&self) -> Option<bool> {
        self.expanded
    }

    #[must_use]
    pub const fn is_disabled(&self) -> bool {
        self.disabled
    }
}

#[derive(Clone, Debug, Default)]
pub struct Query {
    role: Option<Role>,
    name: Option<String>,
    text: Option<String>,
}

impl Query {
    #[must_use]
    pub fn role(role: Role) -> Self {
        Self {
            role: Some(role),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    fn matches(&self, node: &Node) -> bool {
        self.role.is_none_or(|role| role == node.role)
            && self
                .name
                .as_ref()
                .is_none_or(|name| node.name.as_ref() == Some(name))
            && self.text.as_ref().is_none_or(|text| {
                node.name.as_ref() == Some(text) || node.value.as_ref() == Some(text)
            })
    }
}

impl std::fmt::Display for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(role) = self.role {
            write!(f, "role {role:?} ")?;
        }
        if let Some(name) = &self.name {
            write!(f, "name {name:?} ")?;
        }
        if let Some(text) = &self.text {
            write!(f, "text {text:?}")?;
        }
        Ok(())
    }
}

impl Harness {
    #[must_use]
    pub fn find_all(&mut self, query: &Query) -> Vec<Node> {
        let scale = f64::from(self.host().0.cx().scale().factor());
        let update = self.accessibility_tree();
        let lookup: HashMap<AkNodeId, &accesskit::Node> =
            update.nodes.iter().map(|(id, node)| (*id, node)).collect();
        update
            .nodes
            .iter()
            .filter(|(_, node)| node.role() != Role::TextRun && !node.is_hidden())
            .map(|(id, node)| project(*id, node, &lookup, update.focus, scale))
            .filter(|node| query.matches(node))
            .collect()
    }

    #[must_use]
    pub fn query(&mut self, query: &Query) -> Option<Node> {
        self.find_all(query).into_iter().next()
    }

    #[must_use]
    #[track_caller]
    pub fn get(&mut self, query: &Query) -> Node {
        let mut found = self.find_all(query);
        if found.len() == 1 {
            return found.remove(0);
        }
        let tree = self.accessibility_snapshot();
        super::fail(format!(
            "expected one node for {query}, found {}\n{tree}",
            found.len()
        ))
    }

    #[must_use]
    #[track_caller]
    pub fn get_by_role(&mut self, role: Role, name: &str) -> Node {
        self.get(&Query::role(role).name(name))
    }

    #[must_use]
    #[track_caller]
    pub fn get_by_text(&mut self, text: &str) -> Node {
        self.get(&Query::text(text))
    }

    #[must_use]
    pub fn refresh(&mut self, node: &Node) -> Option<Node> {
        self.find_all(&Query::default())
            .into_iter()
            .find(|fresh| fresh.id == node.id)
    }

    #[must_use]
    pub fn focused(&mut self) -> Option<Node> {
        self.find_all(&Query::default())
            .into_iter()
            .find(Node::is_focused)
    }

    #[track_caller]
    pub fn expect_text(&mut self, text: &str) -> &mut Self {
        let query = Query::text(text);
        self.wait_for(&query, |ui| ui.query(&query));
        self
    }

    #[track_caller]
    pub(super) fn labelled(&mut self, label: &str) -> Option<Node> {
        let found = self.find_all(&Query::text(label));
        let controls: Vec<&Node> = found.iter().filter(|node| node.control).collect();
        match (controls.as_slice(), found.as_slice()) {
            ([control], _) => Some((*control).clone()),
            ([], [only]) => Some(only.clone()),
            ([], []) => None,
            _ => {
                let tree = self.accessibility_snapshot();
                super::fail(format!(
                    "{label:?} names {} controls and {} nodes\n{tree}",
                    controls.len(),
                    found.len()
                ))
            }
        }
    }
}

fn project(
    id: AkNodeId,
    node: &accesskit::Node,
    lookup: &HashMap<AkNodeId, &accesskit::Node>,
    focus: AkNodeId,
    scale: f64,
) -> Node {
    Node {
        id,
        role: node.role(),
        name: accessible_name(node, lookup),
        value: node.value().map(str::to_owned),
        numeric: node.numeric_value(),
        bounds: node.bounds().map(|rect| points(rect, scale)),
        focused: id == focus,
        checked: node.toggled().map(|state| state == Toggled::True),
        selected: node.is_selected(),
        expanded: node.is_expanded(),
        disabled: node.is_disabled(),
        control: node.supports_action(Action::Click) || node.supports_action(Action::SetValue),
    }
}

fn accessible_name(
    node: &accesskit::Node,
    lookup: &HashMap<AkNodeId, &accesskit::Node>,
) -> Option<String> {
    if let Some(label) = node.label() {
        return Some(label.to_owned());
    }
    let parts: Vec<&str> = node
        .labelled_by()
        .iter()
        .filter_map(|id| lookup.get(id))
        .filter_map(|target| target.label().or_else(|| target.value()))
        .collect();
    (!parts.is_empty()).then(|| parts.join(" "))
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "accessibility bounds are within display range"
)]
fn points(rect: accesskit::Rect, scale: f64) -> Rect {
    let at = |value: f64| (value / scale) as f32;
    Rect::new(
        Point::new(at(rect.x0), at(rect.y0)),
        at(rect.x1 - rect.x0),
        at(rect.y1 - rect.y0),
    )
}
