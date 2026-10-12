use crate::kernel::{Cx, NodeId, NodeType};
use crate::style::{StyleBinding, TextStyle};
use smallvec::{SmallVec, smallvec};

use crate::layout::engine_drive::NodeHandle;
use crate::layout::forest::LayoutText;
use crate::layout::text_geometry::TextStyleCapture;
use crate::layout::text_style::{
    InheritedText, inherited_direction, inherited_signature, inherited_text_style, node_direction,
    signature_is_rtl,
};
use crate::layout::{LayoutError, LayoutStyle};
use crate::style::locale::Direction;

use super::LayoutEngine;
use super::direction::apply_direction;
use super::overlays::{apply_intrinsic_image_size, apply_reactive_position, apply_style_overlay};

impl LayoutEngine {
    fn hide_children(&mut self, id: NodeId) -> Result<SmallVec<[NodeHandle; 4]>, LayoutError> {
        let parent_handle = self.handle(id)?;
        self.drive.set_children(parent_handle, &[])?;
        self.hidden.insert(id);
        Ok(smallvec![parent_handle])
    }

    pub(super) fn sync_subtree(
        &mut self,
        cx: &Cx,
        id: NodeId,
        inherited: InheritedText<'_>,
    ) -> Result<SmallVec<[NodeHandle; 4]>, LayoutError> {
        let node = cx.get(id).ok_or(LayoutError::NodeMissing(id))?;
        crate::animation_runtime::style::prepare(cx, id);
        self.track_reactive(id, node);
        let kernel_version = node.version();
        let children = SmallVec::<[NodeId; 4]>::from_slice(node.children());
        if matches!(node.kind, NodeType::Fragment) {
            let mut out = SmallVec::<[NodeHandle; 4]>::new();
            for child in &children {
                out.extend(self.sync_subtree(cx, *child, inherited)?);
            }
            return Ok(out);
        }

        let inherited = inherited.under(node);
        let binding = node.get_extension::<StyleBinding>();
        let layered = binding.and_then(|b| TextStyle::layered_over_binding(inherited.style, b));
        let effective = inherited.layered(layered.as_ref());
        publish_text_style(node, effective.style);
        let text_sig_matches = self.synced_text_sig.get(id.key()).copied() == Some(effective.sig);

        let existing_handle = self.core_to_handle.get(id.key()).copied();
        let has_reactive = binding.is_some_and(StyleBinding::has_reactive)
            || node
                .get_extension::<crate::layout::ReactivePosition>()
                .is_some();
        let version_matches = !has_reactive
            && existing_handle.is_some()
            && self.synced_version.get(id.key()).copied() == Some(kernel_version)
            && text_sig_matches;
        if !version_matches {
            self.upsert_node(id, node, inherited.style, inherited.direction)?;
            self.synced_version.insert(id.key(), kernel_version);
        }
        self.synced_text_sig.insert(id.key(), effective.sig);

        if binding.is_some_and(StyleBinding::hides) {
            return self.hide_children(id);
        }

        self.hidden.remove(&id);
        let mut child_handles = SmallVec::<[NodeHandle; 8]>::new();
        for child in &children {
            child_handles.extend(self.sync_subtree(cx, *child, effective)?);
        }
        let parent_handle = self.handle(id)?;
        self.drive.set_children(parent_handle, &child_handles)?;
        Ok(smallvec![parent_handle])
    }

    fn upsert_node(
        &mut self,
        id: NodeId,
        node: &crate::kernel::Node,
        inherited_text: &TextStyle,
        direction: Direction,
    ) -> Result<(), LayoutError> {
        let (intent, text) = Self::layout_inputs_for_node(node, inherited_text, direction);
        let Some(handle) = self.core_to_handle.get(id.key()).copied() else {
            let handle = self.drive.insert_with_kernel(id, intent.data().clone());
            if text.is_some() {
                self.drive.set_text(handle, text)?;
            }
            self.core_to_handle.insert(id.key(), handle);
            return Ok(());
        };
        let style_matches = self
            .drive
            .style_for(handle)
            .is_some_and(|existing| existing == intent.data());
        if !style_matches {
            self.restyled.insert(id);
        }
        if style_matches && text.is_none() && self.drive.text_for(handle).is_none() {
            return Ok(());
        }
        self.drive.set_style(handle, intent.data().clone())?;
        self.drive.set_text(handle, text)
    }

    pub(super) fn apply_dirty_nodes(
        &mut self,
        cx: &Cx,
        dirty: &[NodeId],
    ) -> Result<(), LayoutError> {
        for id in dirty.iter().copied() {
            if cx.get(id).is_some() {
                self.sync_one_node(cx, id)?;
            } else if self.core_to_handle.contains_key(id.key()) {
                self.remove_mirror(id)?;
            }
        }
        for id in dirty.iter().copied() {
            let Some(node) = cx.get(id) else {
                continue;
            };
            self.sync_child_list(cx, id, node)?;
        }
        Ok(())
    }

    pub(super) fn sync_one_node(&mut self, cx: &Cx, id: NodeId) -> Result<(), LayoutError> {
        let node = cx.get(id).ok_or(LayoutError::NodeMissing(id))?;
        crate::animation_runtime::style::prepare(cx, id);
        self.track_reactive(id, node);
        if matches!(node.kind, NodeType::Fragment) {
            return Ok(());
        }
        let inherited = inherited_text_style(cx, id);
        let direction = node_direction(node, inherited_direction(cx, id));
        self.upsert_node(id, node, &inherited, direction)?;
        self.synced_version.insert(id.key(), node.version());

        let layered = node
            .get_extension::<StyleBinding>()
            .and_then(|binding| TextStyle::layered_over_binding(&inherited, binding));
        let effective = layered.as_ref().unwrap_or(&inherited);
        publish_text_style(node, effective);
        let effective_sig = inherited_signature(effective, direction);
        if self.synced_text_sig.get(id.key()).copied() != Some(effective_sig) {
            self.synced_text_sig.insert(id.key(), effective_sig);
            let children = SmallVec::<[NodeId; 8]>::from_slice(node.children());
            for child in children {
                self.cascade_text_styles(cx, child, InheritedText::new(effective, direction))?;
            }
        }
        Ok(())
    }

    fn cascade_text_styles(
        &mut self,
        cx: &Cx,
        id: NodeId,
        inherited: InheritedText<'_>,
    ) -> Result<(), LayoutError> {
        let Some(node) = cx.get(id) else {
            return Ok(());
        };
        let children = SmallVec::<[NodeId; 8]>::from_slice(node.children());
        if matches!(node.kind, NodeType::Fragment) {
            for child in children {
                self.cascade_text_styles(cx, child, inherited)?;
            }
            return Ok(());
        }
        let inherited = inherited.under(node);
        let layered = node
            .get_extension::<StyleBinding>()
            .and_then(|binding| TextStyle::layered_over_binding(inherited.style, binding));
        let effective = inherited.layered(layered.as_ref());
        publish_text_style(node, effective.style);
        let previous = self.synced_text_sig.get(id.key()).copied();
        if previous == Some(effective.sig) {
            return Ok(());
        }
        self.synced_text_sig.insert(id.key(), effective.sig);
        let mirrored = effective.direction.is_rtl() || previous.is_some_and(signature_is_rtl);
        if matches!(node.kind, NodeType::Text { .. }) || mirrored {
            self.upsert_node(id, node, inherited.style, inherited.direction)?;
        }
        for child in children {
            self.cascade_text_styles(cx, child, effective)?;
        }
        Ok(())
    }

    fn layout_inputs_for_node(
        node: &crate::kernel::Node,
        inherited_text: &TextStyle,
        direction: Direction,
    ) -> (LayoutStyle, Option<LayoutText>) {
        let mut intent = node
            .get_extension::<LayoutStyle>()
            .cloned()
            .unwrap_or_default();
        let mut text = match &node.kind {
            NodeType::Text {
                content,
                wrap,
                ellipsis,
            } => Some(LayoutText {
                content: content.clone(),
                style: Some(inherited_text.clone()),
                wrap: *wrap,
                ellipsis: *ellipsis,
            }),
            NodeType::Container | NodeType::Fragment | NodeType::Image { .. } => None,
        };
        if let NodeType::Image { source, .. } = &node.kind {
            intent = apply_intrinsic_image_size(intent, source.width(), source.height());
        }
        let mut text_style = text.as_mut().and_then(|text| text.style.as_mut());
        apply_style_overlay(node, &mut intent, text_style.as_deref_mut());
        if let Some(style) = text_style {
            style.runs = node.get_extension::<crate::style::TextRuns>().cloned();
        }
        apply_direction(intent.data_mut(), direction);
        (apply_reactive_position(node, intent), text)
    }
}

fn publish_text_style(node: &crate::kernel::Node, style: &TextStyle) {
    if let Some(capture) = node.get_extension::<TextStyleCapture>() {
        capture.publish(style);
    }
}
