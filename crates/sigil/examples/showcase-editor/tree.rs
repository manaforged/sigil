use sigil::prelude::*;
use sigil::style::space;
use sigil::widgets::{ListRowSlots, RowTone, TreeNode};

use super::project::{FileId, Folder, GitStatus, PROJECT_NAME, TreeItem};
use super::state::{Activity, Workspace};

const ACTIVITIES: &[(Activity, Icon, &str)] = &[
    (Activity::Explorer, Icon::Layers, "Explorer"),
    (Activity::Search, Icon::Search, "Search"),
    (Activity::SourceControl, Icon::GitBranch, "Source control"),
    (Activity::Run, Icon::Play, "Run and debug"),
];

pub(crate) fn activity_strip(ws: Workspace) -> NodeBuilder {
    ACTIVITIES.iter().fold(
        container().layout(
            LayoutStyle::flex_column()
                .fill_height()
                .shrink(0.0)
                .gap(space(1))
                .align_items(AlignItems::Center),
        ),
        |strip, (activity, glyph, name)| {
            let activity = *activity;
            strip.child(container().children_reactive(move || {
                let base = icon_button("", move || ws.activity.set(activity))
                    .icon(*glyph)
                    .accessible(*name)
                    .size(ControlSize::Default);
                [if ws.activity.get() == activity {
                    base.soft().tone(Tone::Primary)
                } else {
                    base.ghost()
                }]
            }))
        },
    )
}

pub(crate) fn side_bar(ws: Workspace) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill().min_size(0.0, 0.0))
        .bg(Token::SurfaceSubtle)
        .rounded(Radius::Md)
        .child(
            switch(ws.activity)
                .case(Activity::Explorer, move || explorer(ws))
                .case(Activity::SourceControl, move || source_control(ws))
                .case(Activity::Search, || {
                    empty_state("Search", "Type in the command palette to search files.")
                })
                .case(Activity::Run, || {
                    empty_state("Run and debug", "Use Run tests in the top bar.")
                }),
        )
}

fn header(title: &'static str) -> NodeBuilder {
    section_label(title)
}

fn explorer(ws: Workspace) -> NodeBuilder {
    let selected = signal(None);
    effect(move || selected.set(ws.active().map(TreeItem::File)));
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill()
                .min_size(0.0, 0.0)
                .padding_y(space(1)),
        )
        .child(header("EXPLORER"))
        .child(list_row_with(
            false,
            PROJECT_NAME.to_uppercase(),
            ListRowSlots {
                disclosure: Disclosure::Expanded,
                tone: RowTone::Muted,
                ..ListRowSlots::default()
            },
        ))
        .child(
            tree(move || {
                let expanded = ws.expanded.get();
                let dirty = ws.dirty.get();
                level(None, &expanded, &dirty)
            })
            .expanded(ws.expanded)
            .selected(selected)
            .activate(move |item| {
                if let TreeItem::File(file) = item {
                    ws.open_file(*file);
                }
            })
            .label("Explorer")
            .grow(1.0),
        )
}

fn level(
    parent: Option<Folder>,
    expanded: &[TreeItem],
    dirty: &[FileId],
) -> Vec<TreeNode<TreeItem>> {
    let folders = Folder::ALL
        .into_iter()
        .filter(|folder| folder.parent() == parent)
        .map(|folder| folder_node(folder, expanded, dirty));
    let mut files: Vec<FileId> = FileId::ALL
        .into_iter()
        .filter(|file| file.spec().folder == parent)
        .collect();
    files.sort_by_key(|file| file.spec().name);
    folders
        .chain(files.into_iter().map(|file| file_node(file, dirty)))
        .collect()
}

fn folder_node(folder: Folder, expanded: &[TreeItem], dirty: &[FileId]) -> TreeNode<TreeItem> {
    let item = TreeItem::Folder(folder);
    let glyph = if expanded.contains(&item) {
        Icon::FolderOpen
    } else {
        Icon::Folder
    };
    TreeNode::new(item, folder.name())
        .leading(glyph)
        .children(level(Some(folder), expanded, dirty))
}

fn file_node(file: FileId, dirty: &[FileId]) -> TreeNode<TreeItem> {
    let spec = file.spec();
    let node = TreeNode::new(TreeItem::File(file), spec.name).leading(spec.kind.icon());
    let node = if dirty.contains(&file) {
        node.detail("●")
    } else {
        node
    };
    match spec.git {
        Some(git) => node.status(git.letter(), git.tone()),
        None => node,
    }
}

fn source_control(ws: Workspace) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .padding_y(space(1))
                .gap(space(1)),
        )
        .child(header("SOURCE CONTROL"))
        .child(
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .children_reactive(move || {
                    let staged = ws.staged.get();
                    FileId::ALL
                        .into_iter()
                        .filter_map(|file| file.spec().git.map(|git| (file, git)))
                        .map(|(file, git)| change_row(ws, file, git, staged.contains(&file)))
                        .collect::<Vec<_>>()
                }),
        )
}

fn change_row(ws: Workspace, file: FileId, git: GitStatus, staged: bool) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .padding_x(space(2))
                .gap(space(2))
                .align_items(AlignItems::Center),
        )
        .child(chip(git.letter(), git.tone()))
        .child(
            container()
                .layout(LayoutStyle::flex_row().grow(1.0).min_size(0.0, 0.0))
                .pointer_down(move |_| ws.open_file(file))
                .child(text(file.path()).ellipsize().font_size(super::TEXT_SM)),
        )
        .child(
            button(if staged { "Unstage" } else { "Stage" })
                .ghost()
                .size(ControlSize::Compact)
                .activate(move || ws.toggle_stage(file)),
        )
}
