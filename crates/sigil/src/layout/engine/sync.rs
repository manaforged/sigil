use crate::kernel::{Cx, NodeId, NodeType, TextContent};
use crate::style::{StyleBinding, TextStyle};
use smallvec::SmallVec;

use crate::layout::engine_drive::NodeHandle;
use crate::layout::text_geometry::TextStyleCapture;
use crate::layout::text_style::{InheritedText, inherited_text_style, text_style_signature};
use crate::layout::{LayoutError, LayoutStyle};

use super::LayoutEngine;
use super::overlays::{apply_intrinsic_image_size, apply_reactive_position, apply_style_overlay};

impl LayoutEngine {
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
        let children = node
            .children()
            .iter()
            .copied()
            .collect::<SmallVec<[_; 4]>>();
        if matches!(node.kind, NodeType::Fragment) {
            let mut out = SmallVec::<[NodeHandle; 4]>::new();
            for child in &children {
                out.extend(self.sync_subtree(cx, *child, inherited)?);
            }
            return Ok(out);
        }

        let binding = node.get_extension::<StyleBinding>();
        let layered = binding.and_then(|b| TextStyle::layered_over_binding(inherited.style, b));
        let effective = layered.as_ref().map_or(inherited, InheritedText::new);
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
            self.write_synced_node(id, node, inherited.style, kernel_version)?;
        }
        self.synced_text_sig.insert(id.key(), effective.sig);

        if binding.is_some_and(StyleBinding::hides) {
            let parent_handle = self.handle(id)?;
            self.drive.set_children(parent_handle, &[])?;
            let mut out = SmallVec::<[NodeHandle; 4]>::new();
            out.push(parent_handle);
            return Ok(out);
        }

        let mut child_handles = SmallVec::<[NodeHandle; 8]>::new();
        for child in &children {
            child_handles.extend(self.sync_subtree(cx, *child, effective)?);
        }
        let parent_handle = self.handle(id)?;
        self.drive.set_children(parent_handle, &child_handles)?;

        let mut out = SmallVec::<[NodeHandle; 4]>::new();
        out.push(parent_handle);
        Ok(out)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the argument list mirrors the upstream API contract"
    )]
    fn upsert_with_text(
        &mut self,
        id: NodeId,
        existing_handle: Option<NodeHandle>,
        intent: &LayoutStyle,
        text_content: Option<TextContent>,
        text_style: Option<TextStyle>,
        flags: crate::layout::forest::TextFlags,
    ) -> Result<(), LayoutError> {
        if let Some(handle) = existing_handle {
            let style_matches = self
                .drive
                .style_for(handle)
                .is_some_and(|existing| existing == intent.data());
            if !style_matches {
                self.restyled.insert(id);
            }
            let text_matches = text_content.is_none() && self.drive.text_for(handle).is_none();
            if style_matches && text_matches {
                return Ok(());
            }
            let style = intent.data().clone();
            self.drive.set_style(handle, style)?;
            self.drive
                .set_text(handle, text_content, text_style, flags)?;
            return Ok(());
        }

        let style = intent.data().clone();
        let handle = self.drive.insert_with_kernel(id, style);
        if text_content.is_some() {
            self.drive
                .set_text(handle, text_content, text_style, flags)?;
        }
        self.core_to_handle.insert(id.key(), handle);
        Ok(())
    }

    pub(super) fn apply_dirty_nodes(
        &mut self,
        cx: &Cx,
        dirty: &[NodeId],
    ) -> Result<(), LayoutError> {
        for id in dirty.iter().copied() {
            if cx.get(id).is_some() {
                self.sync_one_node(cx, id)?;
            } else {
                self.remove_mirror_if_present(id)?;
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
        let (intent, text_content, text_style, flags) =
            Self::layout_inputs_for_node(node, &inherited);
        let existing = self.core_to_handle.get(id.key()).copied();
        self.upsert_with_text(id, existing, &intent, text_content, text_style, flags)?;
        self.synced_version.insert(id.key(), node.version());

        let layered = node
            .get_extension::<StyleBinding>()
            .and_then(|binding| TextStyle::layered_over_binding(&inherited, binding));
        let effective = layered.as_ref().unwrap_or(&inherited);
        publish_text_style(node, effective);
        let effective_sig = text_style_signature(effective);
        if self.synced_text_sig.get(id.key()).copied() != Some(effective_sig) {
            self.synced_text_sig.insert(id.key(), effective_sig);
            let children = node
                .children()
                .iter()
                .copied()
                .collect::<SmallVec<[_; 8]>>();
            for child in children {
                self.cascade_text_styles(cx, child, InheritedText::new(effective))?;
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
        let children = node
            .children()
            .iter()
            .copied()
            .collect::<SmallVec<[_; 8]>>();
        if matches!(node.kind, NodeType::Fragment) {
            for child in children {
                self.cascade_text_styles(cx, child, inherited)?;
            }
            return Ok(());
        }
        let layered = node
            .get_extension::<StyleBinding>()
            .and_then(|binding| TextStyle::layered_over_binding(inherited.style, binding));
        let effective = layered.as_ref().map_or(inherited, InheritedText::new);
        publish_text_style(node, effective.style);
        if self.synced_text_sig.get(id.key()).copied() == Some(effective.sig) {
            return Ok(());
        }
        self.synced_text_sig.insert(id.key(), effective.sig);
        if matches!(node.kind, NodeType::Text { .. }) {
            let (intent, text_content, text_style, flags) =
                Self::layout_inputs_for_node(node, inherited.style);
            let existing = self.core_to_handle.get(id.key()).copied();
            self.upsert_with_text(id, existing, &intent, text_content, text_style, flags)?;
        }
        for child in children {
            self.cascade_text_styles(cx, child, effective)?;
        }
        Ok(())
    }

    fn write_synced_node(
        &mut self,
        id: NodeId,
        node: &crate::kernel::Node,
        inherited_text: &TextStyle,
        kernel_version: u32,
    ) -> Result<(), LayoutError> {
        let existing_handle = self.core_to_handle.get(id.key()).copied();
        let (intent, text_content, text_style, flags) =
            Self::layout_inputs_for_node(node, inherited_text);
        self.upsert_with_text(
            id,
            existing_handle,
            &intent,
            text_content,
            text_style,
            flags,
        )?;
        self.synced_version.insert(id.key(), kernel_version);
        Ok(())
    }

    fn layout_inputs_for_node(
        node: &crate::kernel::Node,
        inherited_text: &TextStyle,
    ) -> (
        LayoutStyle,
        Option<TextContent>,
        Option<TextStyle>,
        crate::layout::forest::TextFlags,
    ) {
        if matches!(node.kind, NodeType::Container)
            && node.get_extension::<StyleBinding>().is_none()
            && node
                .get_extension::<crate::layout::ReactivePosition>()
                .is_none()
        {
            let intent = node
                .get_extension::<LayoutStyle>()
                .cloned()
                .unwrap_or_default();
            return (
                intent,
                None,
                None,
                crate::layout::forest::TextFlags::default(),
            );
        }
        let mut intent = node
            .get_extension::<LayoutStyle>()
            .cloned()
            .unwrap_or_default();
        let mut text_style = match &node.kind {
            NodeType::Text { .. } => Some(inherited_text.clone()),
            NodeType::Container | NodeType::Fragment | NodeType::Image { .. } => None,
        };
        if let NodeType::Image { source, .. } = &node.kind {
            intent = apply_intrinsic_image_size(intent, source.width(), source.height());
        }
        apply_style_overlay(node, &mut intent, text_style.as_mut());
        intent = apply_reactive_position(node, intent);
        let (text_content, flags) = match &node.kind {
            NodeType::Text {
                content,
                wrap,
                ellipsis,
            } => (
                Some(content.clone()),
                crate::layout::forest::TextFlags {
                    wrap: *wrap,
                    ellipsis: *ellipsis,
                },
            ),
            NodeType::Container | NodeType::Fragment | NodeType::Image { .. } => {
                (None, crate::layout::forest::TextFlags::default())
            }
        };
        (intent, text_content, text_style, flags)
    }

    fn remove_mirror_if_present(&mut self, id: NodeId) -> Result<(), LayoutError> {
        if self.core_to_handle.contains_key(id.key()) {
            self.remove_mirror(id)?;
        }
        Ok(())
    }

    fn sync_child_list(
        &mut self,
        cx: &Cx,
        id: NodeId,
        node: &crate::kernel::Node,
    ) -> Result<(), LayoutError> {
        let parent = if matches!(node.kind, NodeType::Fragment) {
            self.nearest_mirrored_ancestor(cx, id)
        } else {
            Some(id)
        };
        let Some(parent) = parent else {
            return Ok(());
        };
        let Some(parent_node) = cx.get(parent) else {
            return Ok(());
        };
        if parent_node
            .get_extension::<StyleBinding>()
            .is_some_and(StyleBinding::hides)
        {
            self.drive.set_children(self.handle(parent)?, &[])?;
            return Ok(());
        }
        let handles = self.flatten_child_handles(cx, parent_node.children())?;
        self.drive.set_children(self.handle(parent)?, &handles)
    }

    #[allow(
        clippy::unused_self,
        reason = "kept as method for symmetry with sibling sync helpers; will gain &self use when subtree mirror tracking lands"
    )]
    fn nearest_mirrored_ancestor(&self, cx: &Cx, id: NodeId) -> Option<NodeId> {
        let mut current = cx.parent(id);
        while let Some(parent) = current {
            let node = cx.get(parent)?;
            if !matches!(node.kind, NodeType::Fragment) {
                return Some(parent);
            }
            current = cx.parent(parent);
        }
        None
    }

    fn flatten_child_handles(
        &mut self,
        cx: &Cx,
        children: &[NodeId],
    ) -> Result<Vec<NodeHandle>, LayoutError> {
        let mut handles = Vec::new();
        for child in children {
            let node = cx.get(*child).ok_or(LayoutError::NodeMissing(*child))?;
            if matches!(node.kind, NodeType::Fragment) {
                handles.extend(self.flatten_child_handles(cx, node.children())?);
            } else if let Some(handle) = self.core_to_handle.get(child.key()).copied() {
                handles.push(handle);
            } else {
                let text = inherited_text_style(cx, *child);
                let inherited = InheritedText::new(&text);
                handles.extend(self.sync_subtree(cx, *child, inherited)?);
            }
        }
        Ok(handles)
    }
}

fn publish_text_style(node: &crate::kernel::Node, style: &TextStyle) {
    if let Some(capture) = node.get_extension::<TextStyleCapture>() {
        capture.publish(style);
    }
}
