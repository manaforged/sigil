#![cfg(feature = "kittest")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::events_support::routing_helpers;

use std::cell::Cell;
use std::rc::Rc;

use accesskit::{Action, ActionRequest, TreeId};
use sigil::events::*;
use sigil::geometry::Constraint;
use sigil::kernel::{FocusScopeKind, *};
use sigil::layout::*;
use sigil::prelude::Widget;
use sigil::reactive::{Owner, signal};

use routing_helpers::{compute_layout, route_event};

fn dispatch(router: &mut EventRouter, cx: &mut Cx, layout: &LayoutEngine, request: &ActionRequest) {
    let target = NodeId::from_accesskit(request.target_node);
    match request.action {
        Action::Focus if router.is_focus_target(cx, layout, target) => {
            router.set_focus(cx, Some(target));
        }
        Action::Click if router.is_click_target(cx, layout, target) => {
            sigil::events::invoke_node(cx, layout, target);
        }
        _ => {}
    }
}

fn scope_marker(kind: FocusScopeKind) -> impl FnOnce(&mut sigil::kernel::Node) {
    move |node| {
        node.a11y.focus_scope = Some(kind);
    }
}

fn action_request(action: Action, target: NodeId) -> ActionRequest {
    ActionRequest {
        action,
        target_tree: TreeId::ROOT,
        target_node: target.to_accesskit(),
        data: None,
    }
}

#[path = "events_a11y_action_dispatch/dispatch.rs"]
mod dispatch;
#[path = "events_a11y_action_dispatch/hidden.rs"]
mod hidden;
#[path = "events_a11y_action_dispatch/scope.rs"]
mod scope;
