use std::panic::{AssertUnwindSafe, catch_unwind};

use layout::gv::parser::ast::{AttrStmt, AttrStmtTarget, AttributeList, Graph, Stmt};
use layout::gv::{DotParser, GraphBuilder};

use super::bounds::Tight;
use crate::kernel::NodeBuilder;
use crate::style::{Theme, Token};
use crate::widgets::svg_picture::{FONT_FAMILY, hex, picture};

const TIMES: &str = "font-family: Times, serif;";

pub(super) fn figure(source: &str) -> Option<(NodeBuilder, f32)> {
    let graph = parse(source)?;
    let (node, (width, _)) = picture(source, move |theme| render(&graph, theme))?;
    Some((node, width))
}

fn parse(source: &str) -> Option<Graph> {
    catch_unwind(|| DotParser::new(source).process().ok()).ok()?
}

fn render(graph: &Graph, theme: &Theme) -> Option<String> {
    let mut themed = graph.clone();
    themed.list.list.splice(0..0, defaults(theme));
    let svg = catch_unwind(AssertUnwindSafe(|| {
        let mut builder = GraphBuilder::new();
        builder.visit_graph(&themed);
        let mut writer = Tight::new();
        builder.get().do_it(false, false, false, &mut writer);
        writer.finish()
    }))
    .ok()??;
    let text = format!(
        "font-family: '{FONT_FAMILY}', sans-serif; fill: {};",
        hex(theme, Token::OnSurface)
    );
    Some(svg.replace(TIMES, &text))
}

fn defaults(theme: &Theme) -> [Stmt; 2] {
    let attrs = |pairs: &[(&str, Token)]| {
        let mut list = AttributeList::new();
        for (name, token) in pairs {
            list.add_attr(name, &hex(theme, *token));
        }
        list
    };
    let node = attrs(&[("color", Token::Accent8), ("fillcolor", Token::Accent3)]);
    let edge = attrs(&[("color", Token::OnSurfaceMuted)]);
    [
        Stmt::Attribute(AttrStmt::new(AttrStmtTarget::Node, node)),
        Stmt::Attribute(AttrStmt::new(AttrStmtTarget::Edge, edge)),
    ]
}
